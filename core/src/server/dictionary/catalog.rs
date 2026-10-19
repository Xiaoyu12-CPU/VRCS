//! Fixed, optional dictionary downloads. User supplied URLs are never accepted.
//! Downloaded archives are verified before using the normal staged DB importer.

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::{Extension, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tokio::sync::OwnedSemaphorePermit;

use super::super::{
    api_error, db_call, dictionary_import_error, ApiResult, CaptureContext, ContentState,
};
use super::{
    register_import, remove_import, valid_import_id, ImportProgress, RegisterImportError,
    IMPORT_ID_HEADER, IMPORT_PROGRESS_SCALE,
};
use crate::db::Database;
use crate::error::AppError;
use crate::models::DictionarySource;

const SOURCE_URL: &str = "https://github.com/Lyroxide/yomitan-ko-dic/releases/tag/1.0.0";
const LICENSE_URL: &str = "https://creativecommons.org/licenses/by-sa/2.0/kr/";

#[derive(Debug)]
struct CatalogEntry {
    id: &'static str,
    title: &'static str,
    revision: &'static str,
    source_language: &'static str,
    target_language: &'static str,
    url: &'static str,
    bytes: u64,
    sha256: &'static str,
}

// Verified against the actual release archives, not a mutable "latest" URL.
static CATALOG: [CatalogEntry; 2] = [
    CatalogEntry {
        id: "krdict-ko-zh",
        title: "KRDICT ZH",
        revision: "krdict_ko-zh_0924",
        source_language: "ko",
        target_language: "zh",
        url: "https://github.com/Lyroxide/yomitan-ko-dic/releases/download/1.0.0/KO-ZH.KRDICT.zip",
        bytes: 15_746_892,
        sha256: "9aca8ad844c5d28f96bd18354e6a3aa3f239f38a2b3c14b6cfd659469a73b7ff",
    },
    CatalogEntry {
        id: "krdict-ko-ja",
        title: "KRDICT JA",
        revision: "krdict_ko-ja_0924",
        source_language: "ko",
        target_language: "ja",
        url: "https://github.com/Lyroxide/yomitan-ko-dic/releases/download/1.0.0/KO-JA.KRDICT.zip",
        bytes: 16_373_697,
        sha256: "6546d32e1b99c24f85b874bc8e0a103ba85db2afb89e0c065ae1b033a96e1bdb",
    },
];

impl CatalogEntry {
    fn matches_source(&self, source: &DictionarySource) -> bool {
        source.title == self.title
            && source.revision == self.revision
            && source.source_language == self.source_language
            && source.target_language.as_deref() == Some(self.target_language)
    }
}

fn catalog_entry(id: &str) -> Option<&'static CatalogEntry> {
    CATALOG.iter().find(|entry| entry.id == id)
}

pub(crate) async fn dictionary_catalog(
    State(state): State<ContentState>,
) -> ApiResult<Json<Value>> {
    let sources = db_call(Arc::clone(&state.db), |db| db.dictionary_sources())
        .await
        .map_err(dictionary_import_error)?;
    Ok(Json(json!(CATALOG.iter().map(|entry| json!({
        "id": entry.id,
        "title": entry.title,
        "revision": entry.revision,
        "source_language": entry.source_language,
        "target_language": entry.target_language,
        "bytes": entry.bytes,
        "sha256": entry.sha256,
        "source_url": SOURCE_URL,
        "license_url": LICENSE_URL,
        "license": "CC BY-SA 2.0 KR",
        "attribution": "Korean Learners' Dictionary, National Institute of Korean Language; Yomitan conversion by Lyroxide",
        "installed": sources.iter().any(|source| entry.matches_source(source)),
    })).collect::<Vec<_>>())))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InstallRequest {
    catalog_id: String,
}

#[derive(Debug)]
enum InstallError {
    Download,
    Integrity,
    Import(AppError),
}

impl InstallError {
    fn code(&self) -> &'static str {
        match self {
            Self::Download => "dictionary.download_failed",
            Self::Integrity => "dictionary.download_integrity_failed",
            Self::Import(error) => match error {
                AppError::Validation(_) => "dictionary.import_invalid",
                AppError::Conflict(_) => "dictionary.import_conflict",
                AppError::Unavailable(_) => "dictionary.import_unavailable",
                AppError::Storage(_) => "dictionary.import_storage_failed",
                AppError::Internal(_) => "dictionary.import_failed",
            },
        }
    }

    fn into_api_error(self) -> (StatusCode, Json<Value>) {
        match self {
            Self::Download => api_error(
                StatusCode::BAD_GATEWAY,
                "dictionary.download_failed",
                "Could not download the selected dictionary from its fixed source",
            ),
            Self::Integrity => api_error(
                StatusCode::BAD_GATEWAY,
                "dictionary.download_integrity_failed",
                "Dictionary download did not match its pinned size and SHA-256",
            ),
            Self::Import(error) => dictionary_import_error(error),
        }
    }
}

/// A dropped HTTP request must not leave a permanently active progress record.
/// The blocking DB worker keeps its own semaphore permit until it actually ends.
struct Tracking {
    import_id: String,
    finished: bool,
}

impl Drop for Tracking {
    fn drop(&mut self) {
        if !self.finished {
            remove_import(&self.import_id);
        }
    }
}

pub(crate) async fn dictionary_install(
    State(state): State<ContentState>,
    State(context): State<CaptureContext>,
    Extension(import_permit): Extension<Arc<OwnedSemaphorePermit>>,
    headers: HeaderMap,
    Json(request): Json<InstallRequest>,
) -> ApiResult<Json<Value>> {
    let entry = catalog_entry(&request.catalog_id).ok_or_else(|| {
        api_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "dictionary.catalog_not_found",
            "Unknown dictionary catalog entry",
        )
    })?;
    let import_id = headers
        .get(IMPORT_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .filter(|value| valid_import_id(value))
        .ok_or_else(|| {
            api_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "dictionary.import_id_invalid",
                "A valid dictionary import identifier is required",
            )
        })?;
    let progress = register_import(import_id).map_err(|error| match error {
        RegisterImportError::Conflict => api_error(
            StatusCode::CONFLICT,
            "dictionary.import_id_conflict",
            "Dictionary import identifier is already running",
        ),
        RegisterImportError::Capacity => api_error(
            StatusCode::TOO_MANY_REQUESTS,
            "dictionary.import_progress_capacity",
            "Too many dictionary import tasks are being tracked",
        ),
    })?;
    let mut tracking = Tracking {
        import_id: import_id.to_owned(),
        finished: false,
    };
    progress.downloading(0, entry.bytes);
    let result = install(entry, &state, &context, &progress, import_permit).await;
    match result {
        Ok(imported) => {
            progress.phase("complete");
            progress.store(IMPORT_PROGRESS_SCALE, Ordering::Relaxed);
            tracking.finished = true;
            Ok(Json(json!(imported)))
        }
        Err(error) => {
            progress.failed(error.code());
            tracking.finished = true;
            super::super::feature_gate::require_feature(
                &context.config.config.read().expect("config lock"),
                crate::config::FeatureKey::Learning,
            )?;
            Err(error.into_api_error())
        }
    }
}

async fn install(
    entry: &'static CatalogEntry,
    state: &ContentState,
    context: &CaptureContext,
    progress: &Arc<ImportProgress>,
    import_permit: Arc<OwnedSemaphorePermit>,
) -> Result<DictionarySource, InstallError> {
    let sources = db_call(Arc::clone(&state.db), |db| db.dictionary_sources())
        .await
        .map_err(InstallError::Import)?;
    if let Some(installed) = sources
        .into_iter()
        .find(|source| entry.matches_source(source))
    {
        return Ok(installed);
    }

    let mut features = context.config.features_tx.subscribe();
    let archive = tokio::select! {
        biased;
        _ = learning_disabled(&mut features) => return Err(InstallError::Import(AppError::Conflict("Dictionary installation was cancelled".into()))),
        archive = download(entry, progress) => archive?,
    };
    progress.phase("importing");
    progress.store(5_000, Ordering::Relaxed);
    let progress = Arc::clone(progress);
    let db = Arc::clone(&state.db);
    tokio::task::spawn_blocking(move || {
        let _permit = import_permit;
        Database::import_catalog_yomitan_with_progress(
            &db,
            &archive,
            entry.source_language,
            entry.target_language,
            |value| {
                progress.store(
                    5_000 + (value.clamp(0.0, 1.0) * 5_000.0).round() as u32,
                    Ordering::Relaxed,
                )
            },
            || !features.borrow().learning,
        )
    })
    .await
    .map_err(|_| {
        InstallError::Import(AppError::internal(
            "Dictionary installation worker exited unexpectedly",
        ))
    })?
    .map_err(InstallError::Import)
}

async fn learning_disabled(
    features: &mut tokio::sync::watch::Receiver<crate::config::FeatureConfig>,
) {
    loop {
        if !features.borrow().learning {
            return;
        }
        if features.changed().await.is_err() {
            return;
        }
    }
}

async fn download(
    entry: &CatalogEntry,
    progress: &ImportProgress,
) -> Result<Vec<u8>, InstallError> {
    if entry.bytes > crate::yomitan::MAX_ARCHIVE_BYTES as u64 {
        return Err(InstallError::Integrity);
    }
    let client = reqwest::Client::builder()
        .https_only(true)
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(180))
        .redirect(reqwest::redirect::Policy::limited(5))
        .user_agent("VRCS Korean dictionary installer")
        .build()
        .map_err(|_| InstallError::Download)?;
    let response = client
        .get(entry.url)
        .send()
        .await
        .map_err(|_| InstallError::Download)?
        .error_for_status()
        .map_err(|_| InstallError::Download)?;
    if response
        .content_length()
        .is_some_and(|bytes| bytes != entry.bytes)
    {
        return Err(InstallError::Integrity);
    }
    // Allocate from trusted catalog constants, never from response headers.
    let mut archive = Vec::with_capacity(entry.bytes as usize);
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| InstallError::Download)?;
        if chunk.len() as u64 > entry.bytes.saturating_sub(archive.len() as u64) {
            return Err(InstallError::Integrity);
        }
        archive.extend_from_slice(&chunk);
        progress.downloading(archive.len() as u64, entry.bytes);
    }
    progress.phase("verifying");
    progress.store(4_500, Ordering::Relaxed);
    let expected_bytes = entry.bytes;
    let expected_sha256 = entry.sha256;
    let archive = tokio::task::spawn_blocking(move || {
        verify_archive(&archive, expected_bytes, expected_sha256)?;
        Ok::<_, InstallError>(archive)
    })
    .await
    .map_err(|_| InstallError::Integrity)??;
    progress.store(5_000, Ordering::Relaxed);
    Ok(archive)
}

fn verify_archive(
    archive: &[u8],
    expected_bytes: u64,
    expected_sha256: &str,
) -> Result<(), InstallError> {
    if archive.len() as u64 != expected_bytes
        || format!("{:x}", Sha256::digest(archive)) != expected_sha256
    {
        return Err(InstallError::Integrity);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dictionary_catalog_is_fixed_pinned_and_smaller_than_the_import_limit() {
        assert_eq!(CATALOG.len(), 2);
        assert!(catalog_entry("https://example.com/untrusted.zip").is_none());
        for entry in &CATALOG {
            assert!(entry.url.starts_with(
                "https://github.com/Lyroxide/yomitan-ko-dic/releases/download/1.0.0/"
            ));
            assert!(entry.bytes < crate::yomitan::MAX_ARCHIVE_BYTES as u64);
            assert_eq!(entry.sha256.len(), 64);
            assert!(entry.sha256.bytes().all(|byte| byte.is_ascii_hexdigit()));
            assert_eq!(entry.source_language, "ko");
            assert!(matches!(entry.target_language, "zh" | "ja"));
        }
        assert_ne!(CATALOG[0].id, CATALOG[1].id);
        assert_ne!(CATALOG[0].sha256, CATALOG[1].sha256);
        assert!(serde_json::from_value::<InstallRequest>(
            json!({"catalog_id": "krdict-ko-zh", "url": "https://example.com"})
        )
        .is_err());
    }

    #[test]
    fn dictionary_download_verification_rejects_modified_truncated_and_oversized_bytes() {
        let digest = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert!(verify_archive(b"abc", 3, digest).is_ok());
        for archive in [b"abd".as_slice(), b"ab", b"abcd", b""] {
            assert!(matches!(
                verify_archive(archive, 3, digest),
                Err(InstallError::Integrity)
            ));
        }
    }

    #[test]
    fn dictionary_catalog_requires_corrected_metadata_to_consider_a_package_installed() {
        let entry = &CATALOG[0];
        let mut source = DictionarySource {
            id: 1,
            title: entry.title.into(),
            revision: entry.revision.into(),
            source_language: "ja".into(),
            target_language: Some("ko".into()),
            entry_count: 1,
            imported_at: String::new(),
        };
        assert!(!entry.matches_source(&source));
        source.source_language = "ko".into();
        source.target_language = Some("zh".into());
        assert!(entry.matches_source(&source));
        assert!(!CATALOG[1].matches_source(&source));
        source.revision = "different".into();
        assert!(!entry.matches_source(&source));
    }

    async fn smoke_json(request: reqwest::RequestBuilder) -> Value {
        let response = request.send().await.expect("Local Core request failed");
        let status = response.status();
        let body = response.text().await.expect("Could not read Core response");
        assert!(status.is_success(), "Core returned {status}: {body}");
        serde_json::from_str(&body).expect("Core returned invalid JSON")
    }

    async fn smoke_install_over_http(
        client: &reqwest::Client,
        base_url: &str,
        token: &str,
        entry: &CatalogEntry,
        expects_download: bool,
    ) -> Value {
        let import_id = uuid::Uuid::new_v4().to_string();
        let progress_url = format!("{base_url}/api/dictionaries/import/{import_id}");
        let pending = client
            .post(format!("{base_url}/api/dictionaries/install"))
            .bearer_auth(token)
            .header(IMPORT_ID_HEADER, import_id.as_str())
            .json(&json!({"catalog_id": entry.id}))
            .send();
        tokio::pin!(pending);
        let mut previous_progress = 0.0;
        let mut observed_phases = Vec::new();
        let response = loop {
            tokio::select! {
                biased;
                response = &mut pending => break response.expect("Catalog installation request failed"),
                _ = tokio::time::sleep(Duration::from_millis(25)) => {
                    let response = client.get(&progress_url).bearer_auth(token)
                        .timeout(Duration::from_secs(5)).send().await
                        .expect("Catalog progress request failed");
                    // The first poll may precede registration by the POST handler.
                    if response.status() == StatusCode::NOT_FOUND {
                        continue;
                    }
                    assert!(response.status().is_success(), "Progress HTTP status: {}", response.status());
                    let progress: Value = response.json().await.expect("Invalid progress JSON");
                    let value = progress["progress"].as_f64().expect("Missing progress");
                    assert!((0.0..=1.0).contains(&value));
                    assert!(value >= previous_progress, "Progress moved backwards: {progress}");
                    previous_progress = value;
                    let phase = progress["phase"].as_str().expect("Missing progress phase");
                    assert!(matches!(phase, "downloading" | "verifying" | "importing" | "complete" | "failed"));
                    if observed_phases.last().map(String::as_str) != Some(phase) {
                        observed_phases.push(phase.to_owned());
                    }
                    assert_eq!(progress["total_bytes"], entry.bytes);
                    assert!(progress["downloaded_bytes"].as_u64().unwrap() <= entry.bytes);
                }
            }
        };
        let status = response.status();
        let body = response
            .text()
            .await
            .expect("Missing installation response");
        assert!(
            status.is_success(),
            "Installation of {} returned {status}: {body}",
            entry.id
        );
        let source: Value = serde_json::from_str(&body).expect("Invalid installation JSON");
        assert_eq!(source["title"], entry.title);
        assert_eq!(source["revision"], entry.revision);
        assert_eq!(source["source_language"], "ko");
        assert_eq!(source["target_language"], entry.target_language);
        assert_eq!(source["entry_count"], 91_481);
        let progress = smoke_json(client.get(&progress_url).bearer_auth(token)).await;
        assert_eq!(progress["phase"], "complete");
        assert_eq!(progress["progress"], 1.0);
        assert_eq!(progress["total_bytes"], entry.bytes);
        if expects_download {
            assert_eq!(progress["downloaded_bytes"], entry.bytes);
        } else {
            // An already-installed package must return without downloading again.
            assert_eq!(progress["downloaded_bytes"], 0);
        }
        println!("{} HTTP installation: observed {observed_phases:?}; completed source {} with {} entries",
            entry.id, source["id"], source["entry_count"]);
        source
    }

    /// Explicit opt-in only: uses the production HTTP router, fixed GitHub URLs,
    /// real integrity verification, and a temporary database. No ZIP is bundled.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "downloads both real KRDICT archives from GitHub and imports them; run explicitly in Windows CI"]
    async fn dictionary_catalog_real_http_install_smoke() {
        use futures_util::FutureExt;

        let directory = tempfile::tempdir().unwrap();
        let handle = tokio::time::timeout(
            Duration::from_secs(5),
            crate::start(crate::CoreOptions {
                config_path: directory.path().join("config.json"),
                host: Some("127.0.0.1".into()),
                port: Some(0),
                session_token: Some("dictionary-catalog-real-http-smoke".into()),
                vad_model_path: Some(directory.path().join("unused-vad.onnx")),
                asr_model_dir: None,
            }),
        )
        .await
        .expect("Core startup exceeded five seconds")
        .unwrap();
        let base_url = format!("http://{}", handle.address());
        let token = handle.session_token().to_owned();
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(165))
            .build()
            .unwrap();
        let operations = async {
            let catalog = smoke_json(
                client
                    .get(format!("{base_url}/api/dictionaries/catalog"))
                    .bearer_auth(&token),
            )
            .await;
            assert_eq!(catalog.as_array().unwrap().len(), 2);
            assert!(catalog
                .as_array()
                .unwrap()
                .iter()
                .all(|entry| entry["installed"] == false));
            let mut installed = Vec::new();
            for (index, entry) in CATALOG.iter().enumerate() {
                let source = smoke_install_over_http(&client, &base_url, &token, entry, true).await;
                let lookup = smoke_json(
                    client
                        .get(format!("{base_url}/api/dictionary"))
                        .bearer_auth(&token)
                        .query(&[("q", "먹었어요")]),
                )
                .await;
                assert!(
                    lookup
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|word| word["term"] == "먹다" && word["language"] == "ko"),
                    "Korean lookup failed after installing {}: {lookup}",
                    entry.id
                );
                let repeated =
                    smoke_install_over_http(&client, &base_url, &token, entry, false).await;
                assert_eq!(
                    repeated, source,
                    "Repeated installation must preserve the same source"
                );
                let sources = smoke_json(
                    client
                        .get(format!("{base_url}/api/dictionaries"))
                        .bearer_auth(&token),
                )
                .await;
                assert_eq!(sources.as_array().unwrap().len(), index + 1);
                installed.push(source);
            }
            let catalog = smoke_json(
                client
                    .get(format!("{base_url}/api/dictionaries/catalog"))
                    .bearer_auth(&token),
            )
            .await;
            assert!(catalog
                .as_array()
                .unwrap()
                .iter()
                .all(|entry| entry["installed"] == true));

            let old_id = installed[0]["id"].as_i64().unwrap();
            let deleted = smoke_json(
                client
                    .delete(format!("{base_url}/api/dictionaries/{old_id}"))
                    .bearer_auth(&token),
            )
            .await;
            assert_eq!(deleted["deleted"], true);
            let catalog = smoke_json(
                client
                    .get(format!("{base_url}/api/dictionaries/catalog"))
                    .bearer_auth(&token),
            )
            .await;
            let chinese = catalog
                .as_array()
                .unwrap()
                .iter()
                .find(|entry| entry["id"] == CATALOG[0].id)
                .unwrap();
            assert_eq!(chinese["installed"], false);
            let reinstalled =
                smoke_install_over_http(&client, &base_url, &token, &CATALOG[0], true).await;
            assert_ne!(reinstalled["id"].as_i64().unwrap(), old_id);
            let sources = smoke_json(
                client
                    .get(format!("{base_url}/api/dictionaries"))
                    .bearer_auth(&token),
            )
            .await;
            assert_eq!(sources.as_array().unwrap().len(), 2);
        };
        // Bound all network work and still stop the temporary Core after a failed
        // assertion or timeout. Normal unit tests never execute this smoke test.
        let outcome = tokio::time::timeout(
            Duration::from_secs(170),
            std::panic::AssertUnwindSafe(operations).catch_unwind(),
        )
        .await;
        tokio::time::timeout(Duration::from_secs(5), handle.shutdown())
            .await
            .expect("Core shutdown exceeded five seconds")
            .unwrap();
        match outcome {
            Ok(Ok(())) => {}
            Ok(Err(panic)) => std::panic::resume_unwind(panic),
            Err(_) => panic!("Real dictionary HTTP smoke exceeded the 170-second operation budget"),
        }
    }
}
