//! Pack manifest parsing and validation.

use argo_types::{ArgoError, ArgoResult};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Package metadata stored in `pack.toml`.
///
/// ```
/// use argo_vertical_packs::PackManifest;
/// let body = "[pack]\nname='ap-specialist'\nversion='1.0.0'\n\n[[verticals]]\nid='ap'\npath='verticals/ap'\n";
/// let manifest = PackManifest::parse(body).unwrap();
/// assert_eq!(manifest.pack.name, "ap-specialist");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackManifest {
    /// Top-level package identity.
    pub pack: PackMetadata,
    /// Optional fetcher defaults shipped with the pack.
    #[serde(default)]
    pub fetcher: FetcherConfig,
    /// Vertical template entries included in this pack.
    #[serde(default)]
    pub verticals: Vec<VerticalPackEntry>,
}

/// Stable identity for a vertical template pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackMetadata {
    /// Machine-friendly pack name.
    pub name: String,
    /// Semantic version or release tag version.
    pub version: String,
    /// Pack publisher or source organization.
    #[serde(default)]
    pub author: Option<String>,
    /// Human-friendly display name.
    #[serde(default)]
    pub display_name: Option<String>,
    /// Short operator-facing description.
    #[serde(default)]
    pub description: Option<String>,
}

/// Fetcher knobs. `base_url` may include `{name}`, `{version}`, and
/// `{asset}` placeholders for custom hosting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FetcherConfig {
    /// GitHub Releases download base or direct URL template.
    #[serde(default = "default_base_url")]
    pub base_url: String,
    /// Asset filename template.
    #[serde(default = "default_asset_template")]
    pub asset_template: String,
    /// Release tag template.
    #[serde(default = "default_tag_template")]
    pub tag_template: String,
    /// Environment variables checked for a GitHub bearer token.
    #[serde(default = "default_token_env")]
    pub token_env: Vec<String>,
}

impl Default for FetcherConfig {
    fn default() -> Self {
        Self {
            base_url: default_base_url(),
            asset_template: default_asset_template(),
            tag_template: default_tag_template(),
            token_env: default_token_env(),
        }
    }
}

/// A vertical template directory within a pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerticalPackEntry {
    /// Vertical id.
    pub id: String,
    /// Relative path from the pack root.
    pub path: String,
}

impl PackManifest {
    /// Parse and validate a `pack.toml` body.
    pub fn parse(body: &str) -> ArgoResult<Self> {
        let manifest: Self =
            toml::from_str(body).map_err(|e| ArgoError::Schema(format!("pack.toml: {e}")))?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Load and validate `pack.toml` from a pack root directory.
    /// Errors include the absolute file path so the operator can
    /// see *which* pack.toml failed.
    pub fn load(root: &Path) -> ArgoResult<Self> {
        let path = root.join("pack.toml");
        let body = std::fs::read_to_string(&path)
            .map_err(|e| ArgoError::Io(format!("read {}: {e}", path.display())))?;
        let manifest: Self = toml::from_str(&body)
            .map_err(|e| ArgoError::Schema(format!("parse {}: {e}", path.display())))?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Validate identity fields and relative vertical paths.
    pub fn validate(&self) -> ArgoResult<()> {
        validate_token("pack.name", &self.pack.name)?;
        validate_token("pack.version", &self.pack.version)?;
        if self.verticals.is_empty() {
            return Err(ArgoError::Schema(
                "pack.toml must contain at least one [[verticals]] entry".into(),
            ));
        }
        for vertical in &self.verticals {
            validate_token("verticals.id", &vertical.id)?;
            validate_relative_path("verticals.path", &vertical.path)?;
        }
        Ok(())
    }
}

/// Render a template using `{name}`, `{version}`, and `{asset}`.
pub fn render_template(template: &str, name: &str, version: &str, asset: &str) -> String {
    template
        .replace("{name}", name)
        .replace("{version}", version)
        .replace("{asset}", asset)
}

fn validate_token(field: &str, value: &str) -> ArgoResult<()> {
    if value.trim().is_empty() || value.chars().any(char::is_whitespace) {
        return Err(ArgoError::Schema(format!(
            "{field} must be non-empty and whitespace-free"
        )));
    }
    Ok(())
}

fn validate_relative_path(field: &str, value: &str) -> ArgoResult<()> {
    let path = Path::new(value);
    if path.is_absolute() || value.contains("..") || value.trim().is_empty() {
        return Err(ArgoError::Schema(format!(
            "{field} must be a safe relative path"
        )));
    }
    Ok(())
}

fn default_base_url() -> String {
    "https://github.com/mark-hallam/skycot-packs/releases/download".into()
}

fn default_asset_template() -> String {
    "{name}-{version}.tar.gz".into()
}

fn default_tag_template() -> String {
    "{name}-v{version}".into()
}

fn default_token_env() -> Vec<String> {
    vec!["GH_TOKEN".into(), "GITHUB_TOKEN".into()]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_body() -> &'static str {
        "[pack]\nname='recruiter'\nversion='1.0.0'\n\n[[verticals]]\nid='recruiter'\npath='verticals/recruiter'\n"
    }

    #[test]
    fn parses_valid_pack_manifest() {
        let manifest = PackManifest::parse(valid_body()).unwrap();
        assert_eq!(manifest.pack.name, "recruiter");
        assert_eq!(manifest.verticals[0].path, "verticals/recruiter");
    }

    #[test]
    fn rejects_missing_verticals() {
        let err = PackManifest::parse("[pack]\nname='x'\nversion='1'\n").unwrap_err();
        assert!(format!("{err}").contains("verticals"));
    }

    #[test]
    fn rejects_unsafe_vertical_path() {
        let body = "[pack]\nname='x'\nversion='1'\n\n[[verticals]]\nid='x'\npath='../x'\n";
        let err = PackManifest::parse(body).unwrap_err();
        assert!(format!("{err}").contains("safe relative"));
    }

    #[test]
    fn renders_fetch_templates() {
        let out = render_template("{name}/{version}/{asset}", "ap", "1.2.3", "ap.tgz");
        assert_eq!(out, "ap/1.2.3/ap.tgz");
    }

    /// `PackManifest::load` reads `pack.toml` from a pack root and must
    /// surface the absolute path on parse failure so the operator can
    /// see which pack manifest is broken when several are installed.
    #[test]
    fn load_includes_file_path_when_required_field_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let pack_toml = tmp.path().join("pack.toml");
        std::fs::write(&pack_toml, "[pack]\nname='x'\n").unwrap();
        let err = PackManifest::load(tmp.path()).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains(&pack_toml.display().to_string()),
            "expected error to include {} — got {msg}",
            pack_toml.display()
        );
        assert!(
            msg.contains("version"),
            "expected error to name the missing field 'version' — got {msg}"
        );
    }
}
