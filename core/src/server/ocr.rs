use super::{api_error, ApiResult, ServiceContext};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

#[derive(Default, Deserialize)]
#[serde(default)]
pub(super) struct ModelQuery {
    language: crate::config::OcrRecognitionLanguage,
}
pub(super) async fn runtime_status(
    State(state): State<ServiceContext>,
) -> Json<crate::ocr::OcrExecutionStatus> {
    Json(state.config.local_ocr.execution_status())
}

pub(super) async fn model_status(
    State(state): State<ServiceContext>,
    Query(query): Query<ModelQuery>,
) -> ApiResult<Json<crate::ocr::ModelStatus>> {
    state
        .config
        .local_ocr
        .model_status(query.language)
        .await
        .map(Json)
        .map_err(|error| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "ocr.model_status_failed",
                error,
            )
        })
}

pub(super) async fn model_download(
    State(state): State<ServiceContext>,
    Query(query): Query<ModelQuery>,
) -> ApiResult<(StatusCode, Json<crate::ocr::ModelStatus>)> {
    state
        .config
        .local_ocr
        .download_models(query.language)
        .map(|status| (StatusCode::ACCEPTED, Json(status)))
        .map_err(|error| api_error(StatusCode::CONFLICT, "ocr.model_download_failed", error))
}

pub(super) async fn model_delete(
    State(state): State<ServiceContext>,
    Query(query): Query<ModelQuery>,
) -> ApiResult<Json<crate::ocr::ModelStatus>> {
    state
        .config
        .local_ocr
        .delete_models(query.language)
        .await
        .map(Json)
        .map_err(|error| api_error(StatusCode::CONFLICT, "ocr.model_delete_failed", error))
}
