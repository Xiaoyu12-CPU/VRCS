export interface DictionaryEntry {
  term: string;
  language: string;
  definition: string;
  reading?: string | null;
  dictionary?: string | null;
}

export interface DictionarySource {
  id: number;
  title: string;
  revision: string;
  source_language: string;
  target_language: string | null;
  entry_count: number;
  imported_at: string;
}

export interface DictionaryImportProgress {
  progress: number;
  phase?: DictionaryInstallPhase;
  downloaded_bytes?: number;
  total_bytes?: number | null;
  error_code?: string | null;
}

export type DictionaryCatalogId = "krdict-ko-zh" | "krdict-ko-ja";
export type DictionaryInstallPhase = "downloading" | "verifying" | "importing" | "complete" | "failed";

export interface DictionaryCatalogEntry {
  id: DictionaryCatalogId;
  title: string;
  source_language: string;
  target_language: string;
  bytes: number;
  sha256: string;
  source_url: string;
  license_url: string;
  installed: boolean;
}

export interface DictionaryInstallation extends DictionaryImportProgress {
  catalog_id: DictionaryCatalogId;
  phase: DictionaryInstallPhase;
}
