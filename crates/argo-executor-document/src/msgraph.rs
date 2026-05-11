//! Microsoft Graph executor — write/read documents on OneDrive via
//! the Graph v1.0 REST API.
//!
//! Three formats: [`MsGraphFormat::Docx`] / [`MsGraphFormat::Xlsx`]
//! build minimal Open XML; [`MsGraphFormat::UploadOnly`] passes
//! operator-supplied bytes through verbatim. Simple PUT-to-upload
//! semantics (`me/drive/root:/{path}:/content`) with a 4 MB ceiling
//! ([`MAX_UPLOAD_BYTES`]); chunked-session upload is out of scope.
//!
//! Auth: callers supply a bearer token (typically via
//! [`MsGraphClient::from_env`] reading `ARGO_MSGRAPH_ACCESS_TOKEN`).
//! Once a dedicated Microsoft 365 MCP connector lands, that connector
//! will own the OAuth refresh flow and feed tokens here. A 401/403
//! from Graph surfaces as [`MsGraphError::AuthExpired`] with an
//! explicit refresh hint.

use std::io::Write;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 4 MB simple-upload ceiling; Graph requires session upload above
/// this size and that path is out of scope for v1.
pub const MAX_UPLOAD_BYTES: u64 = 4 * 1024 * 1024;

const GRAPH_V1_BASE: &str = "https://graph.microsoft.com/v1.0";
const DOCX_CT: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
const XLSX_CT: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";
const OCTET_CT: &str = "application/octet-stream";

/// Format selector for [`MsGraphExecutor::create`]. Picks the
/// template path and HTTP `Content-Type` header.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MsGraphFormat {
    /// Minimal `.docx` built from `title` + `body`. Single paragraph;
    /// rich layout is out of scope (use `UploadOnly` with a pre-built
    /// docx).
    Docx {
        #[serde(default)]
        title: String,
        #[serde(default)]
        body: String,
    },
    /// Minimal `.xlsx` with one sheet of inline-string cells. First
    /// row is the header.
    Xlsx {
        sheet_name: String,
        #[serde(default)]
        rows: Vec<Vec<String>>,
    },
    /// Bytes uploaded verbatim — operator owns the format.
    UploadOnly,
}

/// File reference returned by [`MsGraphExecutor::create`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MsGraphFile {
    /// Shareable web URL (`https://onedrive.live.com/...`).
    pub web_url: String,
    /// Graph item id for follow-up ops.
    pub item_id: String,
    /// Uploaded file size in bytes.
    pub size_bytes: u64,
}

/// Bearer-token Graph client.
#[derive(Debug, Clone)]
pub struct MsGraphClient {
    pub bearer_token: String,
    pub base_url: String,
}

impl MsGraphClient {
    pub fn new(bearer_token: impl Into<String>) -> Self {
        Self { bearer_token: bearer_token.into(), base_url: GRAPH_V1_BASE.into() }
    }

    /// Read the access token from `ARGO_MSGRAPH_ACCESS_TOKEN`.
    pub fn from_env() -> Result<Self, MsGraphError> {
        match std::env::var("ARGO_MSGRAPH_ACCESS_TOKEN") {
            Ok(t) if !t.trim().is_empty() => Ok(Self::new(t)),
            _ => Err(MsGraphError::AuthMissing {
                hint: "set ARGO_MSGRAPH_ACCESS_TOKEN to a valid Graph bearer token".into(),
            }),
        }
    }

    /// Override the base URL — useful for pointing tests at a mock.
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Construct the `me/drive` (or `drives/{id}`) endpoint URL for
    /// a drive item identified by `path`.
    pub fn endpoint_for_drive_item(&self, drive_id: Option<&str>, path: &str) -> String {
        let path = path.trim_start_matches('/');
        match drive_id {
            Some(id) => format!("{}/drives/{}/root:/{}", self.base_url, id, path),
            None => format!("{}/me/drive/root:/{}", self.base_url, path),
        }
    }
}

/// Document executor targeting Microsoft Graph. `drive_id = None`
/// uses the authenticated user's default drive.
#[derive(Debug, Clone)]
pub struct MsGraphExecutor {
    pub client: MsGraphClient,
    pub drive_id: Option<String>,
}

impl MsGraphExecutor {
    pub fn new(client: MsGraphClient) -> Self {
        Self { client, drive_id: None }
    }

    pub fn with_drive_id(mut self, id: impl Into<String>) -> Self {
        self.drive_id = Some(id.into());
        self
    }

    /// Build the upload payload and PUT it to Graph.
    pub async fn create(
        &self,
        fmt: MsGraphFormat,
        path: &str,
        bytes: &[u8],
    ) -> Result<MsGraphFile, MsGraphError> {
        let payload = build_payload(&fmt, bytes)?;
        if payload.len() as u64 > MAX_UPLOAD_BYTES {
            return Err(MsGraphError::PayloadTooLarge {
                size: payload.len() as u64,
                limit: MAX_UPLOAD_BYTES,
            });
        }
        let url = format!(
            "{}:/content",
            self.client.endpoint_for_drive_item(self.drive_id.as_deref(), path),
        );
        let resp = reqwest::Client::new()
            .put(&url)
            .bearer_auth(&self.client.bearer_token)
            .header("Content-Type", content_type(&fmt))
            .body(payload)
            .send()
            .await
            .map_err(|e| MsGraphError::Request { url: url.clone(), detail: e.to_string() })?;
        let body = consume_response(&url, resp).await?;
        let json: serde_json::Value =
            serde_json::from_slice(&body).map_err(|e| MsGraphError::Parse(e.to_string()))?;
        parse_drive_item(&json)
    }

    /// GET the file bytes for `path`.
    pub async fn read(&self, path: &str) -> Result<Vec<u8>, MsGraphError> {
        let url = format!(
            "{}:/content",
            self.client.endpoint_for_drive_item(self.drive_id.as_deref(), path),
        );
        let resp = reqwest::Client::new()
            .get(&url)
            .bearer_auth(&self.client.bearer_token)
            .send()
            .await
            .map_err(|e| MsGraphError::Request { url: url.clone(), detail: e.to_string() })?;
        consume_response(&url, resp).await
    }
}

async fn consume_response(url: &str, resp: reqwest::Response) -> Result<Vec<u8>, MsGraphError> {
    let status = resp.status();
    let body = resp
        .bytes()
        .await
        .map_err(|e| MsGraphError::Request { url: url.into(), detail: e.to_string() })?;
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Err(classify_auth_error(status.as_u16()));
    }
    if !status.is_success() {
        return Err(MsGraphError::Status {
            status: status.as_u16(),
            body: String::from_utf8_lossy(&body).into_owned(),
        });
    }
    Ok(body.to_vec())
}

/// Build the upload bytes for `fmt`. `UploadOnly` returns `bytes`
/// verbatim; Docx/Xlsx generate minimal Open XML zips.
pub fn build_payload(fmt: &MsGraphFormat, bytes: &[u8]) -> Result<Vec<u8>, MsGraphError> {
    match fmt {
        MsGraphFormat::Docx { title, body } => build_docx(title, body),
        MsGraphFormat::Xlsx { sheet_name, rows } => build_xlsx(sheet_name, rows),
        MsGraphFormat::UploadOnly => Ok(bytes.to_vec()),
    }
}

fn content_type(fmt: &MsGraphFormat) -> &'static str {
    match fmt {
        MsGraphFormat::Docx { .. } => DOCX_CT,
        MsGraphFormat::Xlsx { .. } => XLSX_CT,
        MsGraphFormat::UploadOnly => OCTET_CT,
    }
}

/// Extract the file metadata from a Graph `driveItem` JSON response.
pub fn parse_drive_item(json: &serde_json::Value) -> Result<MsGraphFile, MsGraphError> {
    let web_url = json
        .get("webUrl")
        .and_then(|v| v.as_str())
        .ok_or_else(|| MsGraphError::Parse("response missing webUrl".into()))?
        .to_string();
    let item_id = json
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| MsGraphError::Parse("response missing id".into()))?
        .to_string();
    let size_bytes = json.get("size").and_then(|v| v.as_u64()).unwrap_or(0);
    Ok(MsGraphFile { web_url, item_id, size_bytes })
}

/// Map a 401/403 to [`MsGraphError::AuthExpired`].
pub fn classify_auth_error(status: u16) -> MsGraphError {
    MsGraphError::AuthExpired {
        status,
        hint: "Graph rejected the bearer token (expired or insufficient scope); refresh ARGO_MSGRAPH_ACCESS_TOKEN and retry".into(),
    }
}

fn build_docx(title: &str, body: &str) -> Result<Vec<u8>, MsGraphError> {
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buf);
        let opts = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        write_zip_entry(&mut zip, "[Content_Types].xml", DOCX_CONTENT_TYPES, opts)?;
        write_zip_entry(&mut zip, "_rels/.rels", DOCX_RELS_ROOT, opts)?;
        write_zip_entry(&mut zip, "word/_rels/document.xml.rels", DOCX_RELS_EMPTY, opts)?;
        let document = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t xml:space="preserve">{}</w:t></w:r></w:p><w:p><w:r><w:t xml:space="preserve">{}</w:t></w:r></w:p></w:body></w:document>"#,
            xml_escape(title),
            xml_escape(body),
        );
        write_zip_entry(&mut zip, "word/document.xml", &document, opts)?;
        zip.finish().map_err(zip_err)?;
    }
    Ok(buf.into_inner())
}

fn build_xlsx(sheet_name: &str, rows: &[Vec<String>]) -> Result<Vec<u8>, MsGraphError> {
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buf);
        let opts = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        write_zip_entry(&mut zip, "[Content_Types].xml", XLSX_CONTENT_TYPES, opts)?;
        write_zip_entry(&mut zip, "_rels/.rels", XLSX_RELS_ROOT, opts)?;
        write_zip_entry(&mut zip, "xl/_rels/workbook.xml.rels", XLSX_RELS_WORKBOOK, opts)?;
        let workbook = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="{}" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
            xml_escape(sheet_name),
        );
        write_zip_entry(&mut zip, "xl/workbook.xml", &workbook, opts)?;
        write_zip_entry(&mut zip, "xl/worksheets/sheet1.xml", &build_sheet(rows), opts)?;
        zip.finish().map_err(zip_err)?;
    }
    Ok(buf.into_inner())
}

fn build_sheet(rows: &[Vec<String>]) -> String {
    let mut out = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>"#,
    );
    for (row_idx, row) in rows.iter().enumerate() {
        out.push_str(&format!("<row r=\"{}\">", row_idx + 1));
        for (col_idx, cell) in row.iter().enumerate() {
            out.push_str(&format!(
                "<c r=\"{}\" t=\"inlineStr\"><is><t xml:space=\"preserve\">{}</t></is></c>",
                column_ref(col_idx, row_idx),
                xml_escape(cell),
            ));
        }
        out.push_str("</row>");
    }
    out.push_str("</sheetData></worksheet>");
    out
}

fn column_ref(col_idx: usize, row_idx: usize) -> String {
    let mut col = String::new();
    let mut n = col_idx;
    loop {
        col.insert(0, char::from(b'A' + (n % 26) as u8));
        n /= 26;
        if n == 0 {
            break;
        }
        n -= 1;
    }
    format!("{}{}", col, row_idx + 1)
}

fn write_zip_entry<W: std::io::Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    name: &str,
    body: &str,
    opts: zip::write::FileOptions,
) -> Result<(), MsGraphError> {
    zip.start_file(name, opts).map_err(zip_err)?;
    zip.write_all(body.as_bytes())
        .map_err(|e| MsGraphError::ZipBuild { context: format!("write {name}"), detail: e.to_string() })?;
    Ok(())
}

fn zip_err(e: zip::result::ZipError) -> MsGraphError {
    MsGraphError::ZipBuild { context: "zip writer".into(), detail: e.to_string() }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

const DOCX_CONTENT_TYPES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#;
const DOCX_RELS_ROOT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;
const DOCX_RELS_EMPTY: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"></Relationships>"#;
const XLSX_CONTENT_TYPES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/></Types>"#;
const XLSX_RELS_ROOT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#;
const XLSX_RELS_WORKBOOK: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#;

/// Errors specific to the Microsoft Graph executor.
#[derive(Debug, Error)]
pub enum MsGraphError {
    #[error("msgraph auth missing: {hint}")]
    AuthMissing { hint: String },
    #[error("msgraph auth expired (status {status}): {hint}")]
    AuthExpired { status: u16, hint: String },
    #[error("msgraph request to {url} failed: {detail}")]
    Request { url: String, detail: String },
    #[error("msgraph returned status {status}: {body}")]
    Status { status: u16, body: String },
    #[error("msgraph response parse error: {0}")]
    Parse(String),
    #[error("msgraph zip build {context}: {detail}")]
    ZipBuild { context: String, detail: String },
    #[error("msgraph payload too large: {size} bytes (limit {limit})")]
    PayloadTooLarge { size: u64, limit: u64 },
}

#[cfg(test)]
#[path = "msgraph_tests.rs"]
mod msgraph_tests;
