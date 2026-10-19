import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { localizedError } from "../../app/app-utils";
import { dictionaryApi } from "../../dictionary/api";
import type { DictionaryCatalogEntry, DictionaryCatalogId, DictionaryImportProgress, DictionaryInstallation, DictionarySource } from "../../dictionary/types";

export function useDictionaryActions({
  locale,
  active,
  dictionaries,
  onImport,
  onInstall,
  onDelete,
}: {
  locale: string;
  active: boolean;
  dictionaries: DictionarySource[];
  onImport: (file: File, onProgress?: (progress: number) => void) => Promise<DictionarySource>;
  onInstall: (catalogId: DictionaryCatalogId, onProgress?: (progress: DictionaryImportProgress) => void) => Promise<DictionarySource>;
  onDelete: (id: number) => Promise<void>;
}) {
  const { t } = useTranslation();
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [progress, setProgress] = useState<number | null>(null);
  const [catalog, setCatalog] = useState<DictionaryCatalogEntry[]>([]);
  const [catalogLoading, setCatalogLoading] = useState(true);
  const [catalogError, setCatalogError] = useState(false);
  const [installation, setInstallation] = useState<DictionaryInstallation | null>(null);
  const [messageError, setMessageError] = useState(false);
  const busyRef = useRef(false);
  const catalogRequest = useRef(0);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const loadCatalog = useCallback(async () => {
    const requestId = ++catalogRequest.current;
    setCatalogLoading(true);
    setCatalogError(false);
    try {
      const entries = await dictionaryApi.catalog();
      if (requestId === catalogRequest.current) setCatalog(entries);
    } catch {
      if (requestId === catalogRequest.current) setCatalogError(true);
    } finally {
      if (requestId === catalogRequest.current) setCatalogLoading(false);
    }
  }, []);

  useEffect(() => {
    if (active) void loadCatalog();
  }, [active, dictionaries, loadCatalog]);

  const choose = async (file?: File) => {
    if (!file || busyRef.current) return;
    busyRef.current = true;
    setBusy(true);
    setInstallation(null);
    setMessageError(false);
    setProgress(0);
    setMessage(t("settings.dictionary.importing", { file: file.name }));
    try {
      const imported = await onImport(file, setProgress);
      setMessage(t("settings.dictionary.imported", {
        title: imported.title,
        count: imported.entry_count,
        formatted: new Intl.NumberFormat(locale).format(imported.entry_count),
      }));
    } catch (reason) {
      setMessageError(true);
      setMessage(localizedError(reason, t, "errors.dictionary.import"));
    } finally {
      busyRef.current = false;
      setBusy(false);
      setProgress(null);
      if (fileInputRef.current) fileInputRef.current.value = "";
    }
  };

  const remove = async (dictionary: DictionarySource) => {
    if (busyRef.current) return;
    busyRef.current = true;
    setBusy(true);
    setInstallation(null);
    setMessageError(false);
    try {
      await onDelete(dictionary.id);
      setMessage(t("settings.dictionary.removed", { title: dictionary.title }));
    } catch (reason) {
      setMessageError(true);
      setMessage(localizedError(reason, t, "errors.dictionary.remove"));
    } finally {
      busyRef.current = false;
      setBusy(false);
    }
  };

  const install = async (catalogId: DictionaryCatalogId) => {
    const entry = catalog.find((item) => item.id === catalogId);
    if (busyRef.current || catalogLoading || !entry || entry.installed) return;
    busyRef.current = true;
    setBusy(true);
    setMessage("");
    setMessageError(false);
    setInstallation({ catalog_id: catalogId, phase: "downloading", progress: 0 });
    try {
      const imported = await onInstall(catalogId, (status) => {
        setInstallation({ ...status, catalog_id: catalogId, phase: status.phase ?? "importing" });
      });
      setCatalog((current) => current.map((item) => item.id === catalogId ? { ...item, installed: true } : item));
      setInstallation({ catalog_id: catalogId, phase: "complete", progress: 1 });
      setMessage(t("settings.dictionary.imported", {
        title: imported.title,
        count: imported.entry_count,
        formatted: new Intl.NumberFormat(locale).format(imported.entry_count),
      }));
    } catch (reason) {
      setInstallation((current) => ({
        catalog_id: catalogId,
        progress: current?.progress ?? 0,
        phase: "failed",
      }));
      setMessageError(true);
      setMessage(reason instanceof TypeError
        ? t("errors.dictionary.download_failed")
        : localizedError(reason, t, "errors.dictionary.install"));
    } finally {
      busyRef.current = false;
      setBusy(false);
    }
  };

  return {
    busy,
    message,
    progress,
    catalog,
    catalogLoading,
    catalogError,
    installation,
    messageError,
    fileInputRef,
    loadCatalog,
    install,
    choose,
    remove,
  };
}
