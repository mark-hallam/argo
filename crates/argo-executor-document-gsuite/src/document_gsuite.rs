//! `DocumentGsuite::write` — Drive `files.create` + per-kind
//! `batchUpdate` against Docs / Sheets / Slides. Returns a
//! sha256-stamped [`GsuiteWriteReport`] keyed off the logical request
//! body. Image-URL validation for `Slides` runs at preflight: HTTPS
//! only, `200` HEAD response, `image/*` mime.

use crate::auth::GsuiteAuth;
use crate::document_gsuite_payload::{
    build_sheet_rows, canonical_request_sha256, collect_batch_rejections,
};
use crate::types::{
    GsuiteDocId, GsuiteDocKind, GsuiteRejection, GsuiteSandboxError, GsuiteWriteRequest,
};
use argo_types::{ArgoError, ArgoResult};
use serde_json::{json, Value};

/// Stateless dispatch handle. One instance per process suffices.
pub struct DocumentGsuite {
    auth: GsuiteAuth,
    client: reqwest::Client,
    base_drive: String,
    base_docs: String,
    base_sheets: String,
    base_slides: String,
}

/// Outcome from a successful [`DocumentGsuite::write`]. Surfaced into
/// the worker-action bundle's `result_json` for audit.
#[derive(Debug, Clone)]
pub struct GsuiteWriteReport {
    /// Drive file id of the new doc.
    pub doc_id: GsuiteDocId,
    /// Drive web view link (sharing target for the operator).
    pub web_view_link: String,
    /// Native MIME (e.g. `application/vnd.google-apps.document`).
    pub mime_type: String,
    /// Doc kind (mirrors `mime_type`).
    pub kind: GsuiteDocKind,
    /// Drive-reported creation timestamp (RFC 3339).
    pub created_at: String,
    /// SHA-256 of the canonical-JSON serialisation of the request.
    /// Deterministic — same logical request → same hash.
    pub sha256_of_request: String,
    /// Per-row / per-slide rejections from the `batchUpdate` step.
    pub rejections: Vec<GsuiteRejection>,
}

impl DocumentGsuite {
    /// Construct with the default Google API base URLs.
    #[must_use]
    pub fn new(auth_config: crate::auth::GsuiteAuthConfig) -> Self {
        Self::with_bases(
            GsuiteAuth::new(auth_config),
            reqwest::Client::new(),
            "https://www.googleapis.com",
            "https://docs.googleapis.com",
            "https://sheets.googleapis.com",
            "https://slides.googleapis.com",
        )
    }

    /// Construct with custom client + base URLs (test-only).
    #[doc(hidden)]
    pub fn with_bases(
        auth: GsuiteAuth,
        client: reqwest::Client,
        drive: impl Into<String>,
        docs: impl Into<String>,
        sheets: impl Into<String>,
        slides: impl Into<String>,
    ) -> Self {
        Self {
            auth,
            client,
            base_drive: drive.into(),
            base_docs: docs.into(),
            base_sheets: sheets.into(),
            base_slides: slides.into(),
        }
    }

    /// Borrow the auth cache (test-only helper).
    #[doc(hidden)]
    #[must_use]
    pub fn auth(&self) -> &GsuiteAuth {
        &self.auth
    }

    /// Preflight + create + populate.
    ///
    /// # Errors
    /// [`ArgoError::Schema`] for preflight failures (empty title, bad
    /// image URL); [`ArgoError::Provider`] for any Google API failure.
    pub async fn write(&self, req: GsuiteWriteRequest) -> ArgoResult<GsuiteWriteReport> {
        if req.title.trim().is_empty() {
            return Err(GsuiteSandboxError::EmptyTitle.into());
        }
        let sha = canonical_request_sha256(&req);

        if matches!(req.kind, GsuiteDocKind::Slides) {
            for (i, slide) in req.slides.iter().enumerate() {
                if let Some(url) = slide.image_url.as_deref() {
                    self.validate_image_url(i as u32, url).await?;
                }
            }
        }

        let (doc_id, web_view_link, created_at) = self.drive_create(&req).await?;
        let rejections = match req.kind {
            GsuiteDocKind::Doc => self.populate_doc(&doc_id, &req).await?,
            GsuiteDocKind::Sheet => self.populate_sheet(&doc_id, &req).await?,
            GsuiteDocKind::Slides => self.populate_slides(&doc_id, &req).await?,
        };

        Ok(GsuiteWriteReport {
            doc_id: doc_id.clone(),
            web_view_link,
            mime_type: req.kind.mime_type().to_string(),
            kind: req.kind,
            created_at,
            sha256_of_request: sha,
            rejections,
        })
    }

    async fn drive_create(
        &self,
        req: &GsuiteWriteRequest,
    ) -> ArgoResult<(GsuiteDocId, String, String)> {
        let url = format!(
            "{}/drive/v3/files?fields=id,webViewLink,createdTime,mimeType",
            self.base_drive.trim_end_matches('/')
        );
        let mut body = json!({
            "name": req.title,
            "mimeType": req.kind.mime_type(),
        });
        if let Some(folder) = req.drive_folder_id.as_deref() {
            body["parents"] = json!([folder]);
        }
        let parsed = self.post_with_retry(&url, &body).await?;
        let doc_id = parsed
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| ArgoError::Schema("drive files.create missing id".into()))?
            .to_string();
        let web_view_link = parsed
            .get("webViewLink")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let created_at = parsed
            .get("createdTime")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        Ok((GsuiteDocId(doc_id), web_view_link, created_at))
    }

    async fn populate_doc(
        &self,
        doc_id: &GsuiteDocId,
        req: &GsuiteWriteRequest,
    ) -> ArgoResult<Vec<GsuiteRejection>> {
        let Some(body) = req.doc_body_text.as_deref() else {
            return Ok(Vec::new());
        };
        if body.is_empty() {
            return Ok(Vec::new());
        }
        let url = format!(
            "{}/v1/documents/{}:batchUpdate",
            self.base_docs.trim_end_matches('/'),
            doc_id.0
        );
        let payload = json!({
            "requests": [{
                "insertText": {
                    "location": { "index": 1 },
                    "text": body,
                }
            }]
        });
        let parsed = self.post_with_retry(&url, &payload).await?;
        Ok(collect_batch_rejections(&parsed))
    }

    async fn populate_sheet(
        &self,
        doc_id: &GsuiteDocId,
        req: &GsuiteWriteRequest,
    ) -> ArgoResult<Vec<GsuiteRejection>> {
        let schema = req.sheet_schema.clone().unwrap_or_default();
        if schema.is_empty() {
            return Ok(Vec::new());
        }
        let url = format!(
            "{}/v4/spreadsheets/{}:batchUpdate",
            self.base_sheets.trim_end_matches('/'),
            doc_id.0
        );
        let (rows, rejections) = build_sheet_rows(&schema, &req.sheet_rows);
        let payload = json!({
            "requests": [{
                "appendCells": {
                    "sheetId": 0,
                    "rows": rows,
                    "fields": "userEnteredValue",
                }
            }]
        });
        let parsed = self.post_with_retry(&url, &payload).await?;
        let mut all = collect_batch_rejections(&parsed);
        all.extend(rejections);
        Ok(all)
    }

    async fn populate_slides(
        &self,
        doc_id: &GsuiteDocId,
        req: &GsuiteWriteRequest,
    ) -> ArgoResult<Vec<GsuiteRejection>> {
        if req.slides.is_empty() {
            return Ok(Vec::new());
        }
        let url = format!(
            "{}/v1/presentations/{}:batchUpdate",
            self.base_slides.trim_end_matches('/'),
            doc_id.0
        );
        let mut requests: Vec<Value> = Vec::new();
        for (i, slide) in req.slides.iter().enumerate() {
            let slide_obj_id = format!("argoSlide{i}");
            requests.push(json!({
                "createSlide": {
                    "objectId": slide_obj_id,
                    "slideLayoutReference": { "predefinedLayout": "TITLE_AND_BODY" }
                }
            }));
            requests.push(json!({
                "insertText": {
                    "objectId": format!("argoSlide{i}Title"),
                    "text": slide.title,
                }
            }));
            if !slide.body.is_empty() {
                requests.push(json!({
                    "insertText": {
                        "objectId": format!("argoSlide{i}Body"),
                        "text": slide.body,
                    }
                }));
            }
            if let Some(url) = slide.image_url.as_deref() {
                requests.push(json!({
                    "createImage": {
                        "url": url,
                        "elementProperties": { "pageObjectId": slide_obj_id }
                    }
                }));
            }
        }
        let payload = json!({ "requests": requests });
        let parsed = self.post_with_retry(&url, &payload).await?;
        Ok(collect_batch_rejections(&parsed))
    }

    async fn validate_image_url(&self, slide_index: u32, url: &str) -> ArgoResult<()> {
        if !url.starts_with("https://") {
            return Err(GsuiteSandboxError::InvalidImageUrl {
                slide_index,
                reason: "must use https:// scheme".into(),
            }
            .into());
        }
        let resp = self.client.head(url).send().await.map_err(|e| {
            GsuiteSandboxError::InvalidImageUrl {
                slide_index,
                reason: format!("HEAD probe failed: {e}"),
            }
        })?;
        if !resp.status().is_success() {
            return Err(GsuiteSandboxError::InvalidImageUrl {
                slide_index,
                reason: format!("HEAD returned {}", resp.status()),
            }
            .into());
        }
        let mime = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if !mime.starts_with("image/") {
            return Err(GsuiteSandboxError::InvalidImageUrl {
                slide_index,
                reason: format!("Content-Type '{mime}' is not image/*"),
            }
            .into());
        }
        Ok(())
    }

    async fn post_with_retry(&self, url: &str, body: &Value) -> ArgoResult<Value> {
        for attempt in 0..2 {
            let token = self.auth.acquire_token().await?;
            let resp = self
                .client
                .post(url)
                .bearer_auth(&token)
                .json(body)
                .send()
                .await
                .map_err(|e| ArgoError::Provider(format!("gsuite POST: {e}")))?;
            let status = resp.status();
            if status.as_u16() == 401 && attempt == 0 {
                self.auth.invalidate();
                continue;
            }
            let txt = resp
                .text()
                .await
                .map_err(|e| ArgoError::Provider(format!("gsuite body: {e}")))?;
            if !status.is_success() {
                return Err(ArgoError::Provider(format!(
                    "gsuite returned {status}: {txt}"
                )));
            }
            return serde_json::from_str(&txt)
                .map_err(|e| ArgoError::Schema(format!("gsuite JSON parse: {e} (body: {txt})")));
        }
        Err(ArgoError::Provider(
            "gsuite returned 401 after one retry".into(),
        ))
    }
}
