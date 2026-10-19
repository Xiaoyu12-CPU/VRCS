import { apiErrorFromResponse } from "../core-client/api-error";
import { rawCoreFetch, request } from "../core-client/transport";
import type { DictionaryCatalogEntry, DictionaryCatalogId, DictionaryEntry, DictionaryImportProgress, DictionarySource } from "./types";

function dictionaryImportId(): string {
  if (typeof crypto.randomUUID === "function") return crypto.randomUUID();
  return `${Date.now().toString(36)}-${Math.random().toString(36).slice(2)}`;
}

function delay(milliseconds: number): Promise<void> {
  return new Promise((resolve) => window.setTimeout(resolve, milliseconds));
}

export const dictionaryApi = {
  lookup: (term: string) => request<DictionaryEntry[]>(
    `/api/dictionary?q=${encodeURIComponent(term)}`,
  ),
  dictionaries: () => request<DictionarySource[]>("/api/dictionaries"),
  catalog: () => request<DictionaryCatalogEntry[]>("/api/dictionaries/catalog"),
  installDictionary: async (
    catalogId: DictionaryCatalogId,
    onProgress?: (progress: DictionaryImportProgress) => void,
  ) => {
    const importId = dictionaryImportId();
    const progressAbort = new AbortController();
    let finished = false;
    onProgress?.({ progress: 0, phase: "downloading" });
    const polling = (async () => {
      while (!finished) {
        try {
          const status = await request<DictionaryImportProgress>(
            `/api/dictionaries/import/${encodeURIComponent(importId)}`,
            { signal: progressAbort.signal },
          );
          if (!finished) onProgress?.({ ...status, progress: Math.max(0, Math.min(1, status.progress)) });
        } catch {
          // Core registers the installation after the initial POST reaches it.
        }
        if (!finished) await delay(250);
      }
    })();
    try {
      // Downloads can take longer than the normal JSON request timeout.
      const response = await rawCoreFetch("/api/dictionaries/install", {
        method: "POST",
        headers: { "X-VRCS-Import-Id": importId },
        body: JSON.stringify({ catalog_id: catalogId }),
      });
      if (!response.ok) throw await apiErrorFromResponse(response);
      const imported = (await response.json()) as DictionarySource;
      finished = true;
      onProgress?.({ progress: 1, phase: "complete" });
      return imported;
    } finally {
      finished = true;
      progressAbort.abort();
      await polling;
    }
  },
  importDictionary: async (file: File, onProgress?: (progress: number) => void) => {
    const importId = dictionaryImportId();
    let finished = false;
    onProgress?.(0);
    const polling = (async () => {
      while (!finished) {
        try {
          const status = await request<DictionaryImportProgress>(
            `/api/dictionaries/import/${encodeURIComponent(importId)}`,
          );
          onProgress?.(Math.max(0, Math.min(1, status.progress)));
        } catch {
          // The progress endpoint can briefly return 404 before Core registers the uploaded job.
        }
        await delay(100);
      }
    })();
    try {
      const response = await rawCoreFetch("/api/dictionaries/import", {
        method: "POST",
        headers: {
          "Content-Type": "application/zip",
          "X-VRCS-Import-Id": importId,
        },
        body: file,
      });
      if (!response.ok) throw await apiErrorFromResponse(response);
      onProgress?.(1);
      return (await response.json()) as DictionarySource;
    } finally {
      finished = true;
      await polling;
    }
  },
  deleteDictionary: (id: number) => request<{ deleted: boolean }>(
    `/api/dictionaries/${id}`,
    { method: "DELETE" },
  ),
};
