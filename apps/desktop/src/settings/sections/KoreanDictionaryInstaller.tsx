import { Check, Download, LoaderCircle, RotateCcw } from "lucide-react";
import { useTranslation } from "react-i18next";

import type { DictionaryCatalogEntry, DictionaryCatalogId, DictionaryInstallation } from "../../dictionary/types";

export interface KoreanDictionaryInstallerState {
  catalog: DictionaryCatalogEntry[];
  loading: boolean;
  error: boolean;
  installation: DictionaryInstallation | null;
  onInstall: (catalogId: DictionaryCatalogId) => Promise<void>;
  onReload: () => Promise<void>;
}

const choices = [
  { id: "krdict-ko-zh", language: "chinese" },
  { id: "krdict-ko-ja", language: "japanese" },
] as const;

export function KoreanDictionaryInstaller({
  catalog,
  loading,
  error,
  installation,
  busy,
  onInstall,
  onReload,
}: KoreanDictionaryInstallerState & { busy: boolean }) {
  const { t } = useTranslation();
  const source = catalog[0];

  return (
    <div className="dictionary-quick-install">
      <div className="dictionary-quick-install-heading">
        <strong>{t("settings.dictionary.quickInstall.title")}</strong>
        <p>{t("settings.dictionary.quickInstall.description")}</p>
      </div>
      <div className="dictionary-quick-install-options">
        {choices.map(({ id, language }, index) => {
          const entry = catalog.find((item) => item.id === id);
          const current = installation?.catalog_id === id ? installation : null;
          const installed = entry?.installed ?? false;
          const working = busy && current !== null && current.phase !== "complete" && current.phase !== "failed";
          const failed = current?.phase === "failed" && !installed;
          const name = t(`settings.dictionary.quickInstall.${language}`);
          const progress = Math.max(0, Math.min(1, current?.progress ?? 0));
          const percentage = Math.round(progress * 100);

          return (
            <div className="dictionary-quick-install-option" key={id}>
              <div className="dictionary-quick-install-name"><strong>{name}</strong>{index === 0 && <span>{t("settings.dictionary.quickInstall.recommended")}</span>}</div>
              <p>{t(`settings.dictionary.quickInstall.${language}Description`)}</p>
              <button
                className={index === 0 ? "primary-button" : "secondary-button"}
                type="button"
                disabled={busy || loading || !entry || installed}
                aria-label={t(installed ? "settings.dictionary.quickInstall.installedNamed" : failed ? "settings.dictionary.quickInstall.retryNamed" : "settings.dictionary.quickInstall.installNamed", { name })}
                onClick={() => void onInstall(id)}
              >
                {installed ? <Check size={15} /> : working ? <LoaderCircle size={15} className="spin" /> : failed ? <RotateCcw size={15} /> : <Download size={15} />}
                {installed
                  ? t("settings.dictionary.quickInstall.installed")
                  : working
                    ? t("settings.dictionary.quickInstall.installing")
                    : t(failed ? "settings.dictionary.quickInstall.retry" : "settings.dictionary.quickInstall.install")}
              </button>
              {working && (
                <div className="dictionary-import-progress">
                  <div
                    className="dictionary-progress-track"
                    role="progressbar"
                    aria-label={t("settings.dictionary.quickInstall.progressLabel", { name })}
                    aria-valuemin={0}
                    aria-valuemax={100}
                    aria-valuenow={percentage}
                    aria-valuetext={`${t(`settings.dictionary.quickInstall.phases.${current.phase}`)} ${percentage}%`}
                  ><span style={{ transform: `scaleX(${progress})` }} /></div>
                  <span aria-hidden="true">{percentage}%</span>
                </div>
              )}
              {current && <span className={`dictionary-install-status${failed ? " error" : ""}`} role="status" aria-live="polite">{t(`settings.dictionary.quickInstall.phases.${current.phase}`)}</span>}
            </div>
          );
        })}
      </div>
      {loading && <p className="dictionary-catalog-feedback" role="status">{t("settings.dictionary.quickInstall.loading")}</p>}
      {error && (
        <div className="dictionary-catalog-feedback error" role="alert">
          <span>{t("settings.dictionary.quickInstall.catalogError")}</span>
          <button className="secondary-button" type="button" disabled={loading || busy} onClick={() => void onReload()}>{t("settings.dictionary.quickInstall.reload")}</button>
        </div>
      )}
      <p className="dictionary-source-credit">{t("settings.dictionary.quickInstall.attribution")}</p>
      <details className="dictionary-source-details">
        <summary>{t("settings.dictionary.quickInstall.sources")}</summary>
        <div>
          <a href="https://krdict.korean.go.kr/" target="_blank" rel="noreferrer">{t("settings.dictionary.quickInstall.originalSource")}</a>
          {source && <a href={source.source_url} target="_blank" rel="noreferrer">{t("settings.dictionary.quickInstall.convertedSource")}</a>}
          <a href={source?.license_url ?? "https://creativecommons.org/licenses/by-sa/2.0/kr/"} target="_blank" rel="noreferrer">CC BY-SA 2.0 KR</a>
        </div>
      </details>
    </div>
  );
}
