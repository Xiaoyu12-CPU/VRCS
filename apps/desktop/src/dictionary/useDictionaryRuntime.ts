import { useCallback, useEffect, useState } from "react";

import type { ReportRuntimeError } from "../core-client/useRuntimeErrors";
import { dictionaryApi } from "./api";
import type { DictionaryCatalogId, DictionaryImportProgress, DictionarySource } from "./types";

export function useDictionaryRuntime({
  active,
  coreConfigured,
  clearErrorFrom,
  reportError,
}: {
  active: boolean;
  coreConfigured: boolean;
  clearErrorFrom: (source: string) => void;
  reportError: ReportRuntimeError;
}) {
  const [sources, setSources] = useState<DictionarySource[]>([]);

  const refresh = useCallback(async () => {
    if (!coreConfigured) return;
    try {
      setSources(await dictionaryApi.dictionaries());
      clearErrorFrom("dictionary");
    } catch (reason) {
      reportError(reason, "errors.dictionary.list", "dictionary");
    }
  }, [clearErrorFrom, coreConfigured, reportError]);

  useEffect(() => {
    if (active) void refresh();
  }, [active, refresh]);

  const importFile = useCallback(async (
    file: File,
    onProgress?: (progress: number) => void,
  ) => {
    const imported = await dictionaryApi.importDictionary(file, onProgress);
    await refresh();
    return imported;
  }, [refresh]);

  const deleteById = useCallback(async (id: number) => {
    await dictionaryApi.deleteDictionary(id);
    await refresh();
  }, [refresh]);

  const installCatalog = useCallback(async (
    catalogId: DictionaryCatalogId,
    onProgress?: (progress: DictionaryImportProgress) => void,
  ) => {
    try {
      return await dictionaryApi.installDictionary(catalogId, onProgress);
    } finally {
      await refresh();
    }
  }, [refresh]);

  return { sources, refresh, importFile, installCatalog, deleteById };
}
