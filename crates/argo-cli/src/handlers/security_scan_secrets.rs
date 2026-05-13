//! `argo security scan-secrets` — CI parity for the pre-commit
//! secret-scan hook. Reads paths (files or directories), runs every
//! pattern in [`patterns`] against each line, suppresses matches in
//! `.skycot/secret-scan-allowlist.toml`, prints redacted hits, and
//! exits non-zero on any unsuppressed match. The pre-commit hook
//! (`scripts/git-hooks/pre-commit-secret-scan.sh`) is the human-side
//! line of defence; this command is the authoritative CI gate that
//! catches hook bypasses (`--no-verify`, missing install, etc.).
//! Pattern catalogue lives in `docs/skycot/security/secret-patterns.md`.

use argo_types::{ArgoError, ArgoResult};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::cli::cli_security_cmds::SecretScanArgs;

/// One named regex pattern that flags a likely credential.
/// `id` is the stable identifier used by the allowlist; `human` is
/// what the operator sees in the redacted match report.
pub struct Pattern {
    pub id: &'static str,
    pub human: &'static str,
    pub regex: Regex,
}

/// One emitted finding. JSON-serializable so CI can pipe results
/// into dashboards. The raw match never leaves the scanner — only
/// the redaction and the SHA-256 do — so a finding is safe to log
/// even if the underlying file was a real leak.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Finding {
    pub file: String,
    pub line: usize,
    pub regex_id: String,
    pub redacted: String,
    pub match_hash: String,
}

/// Parsed operator allowlist (`.skycot/secret-scan-allowlist.toml`).
/// Designed for surgical suppression: an entry is identified by the
/// tuple `(file_glob, regex_id, match_hash)`, so adding a fixture to
/// the allowlist suppresses exactly that one match without weakening
/// the scan everywhere else.
#[derive(Debug, Default, Clone, Deserialize)]
pub struct Allowlist {
    #[serde(default, rename = "entry")]
    pub entries: Vec<AllowEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AllowEntry {
    pub file_glob: String,
    pub regex_id: String,
    pub match_hash: String,
    /// Free-form note explaining why the match was allowlisted.
    /// Surfaced via `Debug` for operator triage; never read in
    /// the suppression code path.
    #[serde(default)]
    #[allow(dead_code)]
    pub reason: Option<String>,
}

/// The full set of patterns the scanner checks. Adding a new
/// category is a 1-line tuple here plus a documentation row in
/// `docs/skycot/security/secret-patterns.md`. The pattern set
/// intentionally biases toward false-positives; the allowlist is
/// the operator escape valve.
pub fn patterns() -> Vec<Pattern> {
    // Patterns are static literals; the unit tests exercise every
    // entry, so a malformed regex fails CI before reaching a user.
    // The `filter_map` builder is graceful (no `expect`/`unwrap`) so
    // any future bad pattern degrades coverage rather than crashing.
    const RAW: &[(&str, &str, &str)] = &[
        ("openai_sk", "OpenAI API key", r"sk-[A-Za-z0-9_\-]{20,}"),
        ("anthropic_key", "Anthropic API key", r"sk-ant-[A-Za-z0-9_\-]{20,}"),
        ("aws_access_key_id", "AWS access key id", r"AKIA[0-9A-Z]{16}"),
        ("stripe_live_secret", "Stripe live key", r"(?:sk|rk)_live_[A-Za-z0-9]{20,}"),
        ("slack_token", "Slack token", r"xox[abprs]-[A-Za-z0-9-]{10,}"),
        ("github_pat", "GitHub PAT", r"gh[pousr]_[A-Za-z0-9]{36,}"),
        ("private_key_header", "PEM private-key header", r"-----BEGIN (?:RSA |DSA |EC |OPENSSH |PGP )?PRIVATE KEY-----"),
        ("jwt_long", "JWT-shaped token", r"eyJ[A-Za-z0-9_\-]{20,}\.[A-Za-z0-9_\-]{20,}\.[A-Za-z0-9_\-]{20,}"),
        ("argo_bearer_token", "Argo gateway bearer token", r"ARGO_GATEWAY_BEARER_TOKEN\s*[:=]\s*[\x22']?[A-Za-z0-9_\-]{16,}"),
        ("argo_admin_token", "Argo admin token", r"ARGO_ADMIN_TOKEN\s*[:=]\s*[\x22']?[A-Za-z0-9_\-]{16,}"),
    ];
    RAW.iter()
        .filter_map(|(id, human, src)| Regex::new(src).ok().map(|regex| Pattern { id, human, regex }))
        .collect()
}

/// Redact a match preview so the operator can identify it without
/// leaking the secret. `sk-1234abcd...wxyz` keeps the first 4 and
/// last 4 chars; everything shorter than 12 chars is fully masked.
pub fn redact(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() < 12 {
        return "*".repeat(chars.len());
    }
    let head: String = chars.iter().take(4).collect();
    let tail: String = chars.iter().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect();
    format!("{head}...{tail}")
}

/// SHA-256 of the raw match, hex-encoded. Stable identifier for
/// allowlist entries — operators paste this hash into the TOML when
/// they confirm a finding is a known-safe test fixture.
pub fn hash_match(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    hex::encode(h.finalize())
}

/// Scan a single in-memory buffer (used by the scanner for files
/// and by tests for embedded fixtures). Returns one [`Finding`] per
/// suppressed-or-not match; the caller filters by the allowlist.
pub fn scan_buffer(file: &str, body: &str, patterns: &[Pattern]) -> Vec<Finding> {
    let mut out = Vec::new();
    for (idx, line) in body.lines().enumerate() {
        for pat in patterns {
            for m in pat.regex.find_iter(line) {
                let raw = m.as_str();
                out.push(Finding {
                    file: file.to_string(),
                    line: idx + 1,
                    regex_id: pat.id.to_string(),
                    redacted: redact(raw),
                    match_hash: hash_match(raw),
                });
            }
        }
    }
    out
}

/// `(file_glob, regex_id, match_hash)` tuple match.
/// Globs are evaluated with a simple `*` / `**` walker — full glob
/// is overkill for an allowlist and pulling in another crate just
/// for this would inflate the build graph. `**` matches any
/// number of path components; `*` matches one component.
pub fn is_allowlisted(finding: &Finding, allow: &Allowlist) -> bool {
    allow.entries.iter().any(|e| {
        e.regex_id == finding.regex_id
            && e.match_hash == finding.match_hash
            && glob_match(&e.file_glob, &finding.file)
    })
}

fn glob_match(glob: &str, path: &str) -> bool {
    let g: Vec<&str> = glob.split('/').collect();
    let p: Vec<&str> = path.split('/').collect();
    glob_match_segments(&g, &p)
}

fn glob_match_segments(g: &[&str], p: &[&str]) -> bool {
    match (g.first(), p.first()) {
        (None, None) => true,
        (None, Some(_)) => false,
        (Some(&"**"), _) => {
            if g.len() == 1 {
                return true;
            }
            for i in 0..=p.len() {
                if glob_match_segments(&g[1..], &p[i..]) {
                    return true;
                }
            }
            false
        }
        (Some(_), None) => false,
        (Some(gs), Some(ps)) => segment_match(gs, ps) && glob_match_segments(&g[1..], &p[1..]),
    }
}

fn segment_match(gs: &str, ps: &str) -> bool {
    if gs == "*" {
        return true;
    }
    if !gs.contains('*') {
        return gs == ps;
    }
    let parts: Vec<&str> = gs.split('*').collect();
    let mut cursor = 0;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if i == 0 && !ps[cursor..].starts_with(part) {
            return false;
        }
        match ps[cursor..].find(part) {
            Some(idx) => cursor += idx + part.len(),
            None => return false,
        }
    }
    if let Some(last) = parts.last() {
        if !last.is_empty() && !ps.ends_with(last) {
            return false;
        }
    }
    true
}

/// Load an allowlist from disk. Missing file is not an error: the
/// scanner runs with an empty allowlist, which is the strict
/// default. Malformed TOML, however, is a fatal error — the
/// operator needs immediate feedback when the allowlist is broken.
pub fn load_allowlist(path: &Path) -> ArgoResult<Allowlist> {
    if !path.exists() {
        return Ok(Allowlist::default());
    }
    let body = std::fs::read_to_string(path)
        .map_err(|e| ArgoError::Schema(format!("read allowlist {}: {e}", path.display())))?;
    toml::from_str(&body)
        .map_err(|e| ArgoError::Schema(format!("parse allowlist {}: {e}", path.display())))
}

/// Recurse into directories, returning every text file we should
/// scan. We skip `.git/` (binary objects, no payoff) and any path
/// whose first 1 KiB contains a NUL byte (likely binary). Large
/// repos with many binary blobs would otherwise eat a lot of CPU
/// for zero security value.
fn walk_paths(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();
    for root in roots {
        walk_one(root, &mut out, &mut seen);
    }
    out
}

fn walk_one(path: &Path, out: &mut Vec<PathBuf>, seen: &mut HashSet<PathBuf>) {
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    if !seen.insert(canonical.clone()) {
        return;
    }
    let Ok(meta) = std::fs::metadata(path) else {
        return;
    };
    if meta.is_file() {
        if is_text_file(path) {
            out.push(path.to_path_buf());
        }
        return;
    }
    if meta.is_dir() {
        if path.file_name().map(|n| n == ".git").unwrap_or(false) {
            return;
        }
        let Ok(entries) = std::fs::read_dir(path) else {
            return;
        };
        for entry in entries.flatten() {
            walk_one(&entry.path(), out, seen);
        }
    }
}

fn is_text_file(path: &Path) -> bool {
    use std::io::Read;
    let Ok(mut f) = std::fs::File::open(path) else {
        return false;
    };
    let mut buf = [0u8; 1024];
    let n = f.read(&mut buf).unwrap_or(0);
    !buf[..n].contains(&0)
}

/// Public entry point invoked from the CLI dispatch and from
/// integration tests in the sibling `*_tests.rs` module.
pub fn scan_paths(
    roots: &[PathBuf],
    allow: &Allowlist,
    patterns: &[Pattern],
) -> ArgoResult<Vec<Finding>> {
    let files = walk_paths(roots);
    let mut findings = Vec::new();
    for file in files {
        let Ok(body) = std::fs::read_to_string(&file) else {
            continue;
        };
        let display = file.display().to_string();
        for f in scan_buffer(&display, &body, patterns) {
            if !is_allowlisted(&f, allow) {
                findings.push(f);
            }
        }
    }
    Ok(findings)
}

/// CLI dispatch — invoked from `security_audit::run` for the
/// `SecurityCmd::ScanSecrets` variant. Exits non-zero when any
/// unsuppressed finding survives, so CI can wire this up as a
/// blocking gate without further glue.
pub fn run(args: SecretScanArgs) -> ArgoResult<()> {
    let SecretScanArgs { paths, allowlist, json } = args;
    if paths.is_empty() {
        return Err(ArgoError::Schema(
            "argo security scan-secrets: at least one path required".into(),
        ));
    }
    let allow_path = allowlist.unwrap_or_else(|| PathBuf::from(".skycot/secret-scan-allowlist.toml"));
    let allow = load_allowlist(&allow_path)?;
    let pats = patterns();
    let findings = scan_paths(&paths, &allow, &pats)?;
    if json {
        let rendered = serde_json::to_string(&findings)
            .map_err(|e| ArgoError::Schema(format!("render findings: {e}")))?;
        println!("{rendered}");
    } else {
        for f in &findings {
            let pat = pats.iter().find(|p| p.id == f.regex_id);
            let human = pat.map(|p| p.human).unwrap_or(&f.regex_id);
            println!(
                "{}:{}: [{}] {} → {}",
                f.file, f.line, f.regex_id, human, f.redacted
            );
        }
        if !findings.is_empty() {
            eprintln!(
                "argo: {} secret-like match(es) found; add false positives to {}",
                findings.len(),
                allow_path.display()
            );
        }
    }
    if !findings.is_empty() {
        std::process::exit(1);
    }
    Ok(())
}
