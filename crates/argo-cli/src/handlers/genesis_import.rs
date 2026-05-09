//! `argo genesis import` — convert an existing prompt source
//! (LangChain Python file, markdown, or raw text) into a vertical
//! scaffold (`vertical.toml` + `system-prompt.md` + empty
//! `eval-seed.toml` + empty `memory-schema.toml`).
//!
//! The scaffold is intentionally minimal: Genesis's normal
//! generators (`genesis revise`, action-suggestor, eval-seed,
//! memory-schema-builder) refine it later. v0 reads local paths
//! only — URL fetch is deferred to a follow-up that needs a
//! security review.

use argo_types::{ArgoError, ArgoResult};
use regex::Regex;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct GenesisImportArgs {
    pub source: PathBuf,
    pub name: String,
    pub format: ImportFormat,
    pub output: PathBuf,
    pub json: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportFormat {
    Auto,
    Langchain,
    Markdown,
    Raw,
}

impl std::str::FromStr for ImportFormat {
    type Err = ArgoError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "langchain" => Ok(Self::Langchain),
            "markdown" | "md" => Ok(Self::Markdown),
            "raw" | "txt" | "text" => Ok(Self::Raw),
            other => Err(ArgoError::Schema(format!(
                "unknown --format '{other}'; expected auto|langchain|markdown|raw"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct ImportOutcome {
    vertical_id: String,
    output_dir: String,
    format: &'static str,
    prompt_bytes: usize,
}

pub fn handle_import(args: GenesisImportArgs) -> ArgoResult<()> {
    let outcome = import_to_dir(&args)?;
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&outcome).unwrap_or_else(|_| "{}".into())
        );
    } else {
        println!(
            "imported {} from {} into {} ({} bytes prompt)",
            outcome.vertical_id,
            args.source.display(),
            outcome.output_dir,
            outcome.prompt_bytes
        );
        println!(
            "next: review system-prompt.md, then `argo genesis refine {}` to flesh out actions/memory/eval.",
            outcome.vertical_id
        );
    }
    Ok(())
}

fn import_to_dir(args: &GenesisImportArgs) -> ArgoResult<ImportOutcome> {
    validate_name(&args.name)?;
    let source = read_source(&args.source)?;
    let resolved = resolve_format(args.format, &args.source);
    let prompt = extract_prompt(resolved, &source, &args.source)?;
    let dest = args.output.join(&args.name);
    if dest.exists() {
        return Err(ArgoError::Schema(format!(
            "vertical '{}' already exists at {} — pick a different --name or remove the directory first",
            args.name,
            dest.display()
        )));
    }
    std::fs::create_dir_all(&dest)?;
    write_scaffold(&dest, &args.name, &prompt)?;
    Ok(ImportOutcome {
        vertical_id: args.name.clone(),
        output_dir: dest.display().to_string(),
        format: format_label(resolved),
        prompt_bytes: prompt.len(),
    })
}

fn read_source(path: &Path) -> ArgoResult<String> {
    if !path.exists() {
        return Err(ArgoError::Provider(format!(
            "source not found: {}",
            path.display()
        )));
    }
    if !path.is_file() {
        return Err(ArgoError::Provider(format!(
            "source is not a regular file: {}",
            path.display()
        )));
    }
    std::fs::read_to_string(path)
        .map_err(|e| ArgoError::Io(format!("read {}: {e}", path.display())))
}

fn resolve_format(requested: ImportFormat, source: &Path) -> ImportFormat {
    if requested != ImportFormat::Auto {
        return requested;
    }
    let ext = source
        .extension()
        .and_then(|s| s.to_str())
        .map(str::to_ascii_lowercase);
    match ext.as_deref() {
        Some("py") => ImportFormat::Langchain,
        Some("md" | "markdown") => ImportFormat::Markdown,
        _ => ImportFormat::Raw,
    }
}

fn format_label(format: ImportFormat) -> &'static str {
    match format {
        ImportFormat::Auto => "auto",
        ImportFormat::Langchain => "langchain",
        ImportFormat::Markdown => "markdown",
        ImportFormat::Raw => "raw",
    }
}

fn extract_prompt(format: ImportFormat, source: &str, path: &Path) -> ArgoResult<String> {
    let body = match format {
        ImportFormat::Auto => unreachable!("resolve_format collapses Auto"),
        ImportFormat::Langchain => extract_langchain(source, path)?,
        ImportFormat::Markdown => extract_markdown(source),
        ImportFormat::Raw => source.to_string(),
    };
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return Err(ArgoError::Schema(format!(
            "extracted prompt from {} is empty",
            path.display()
        )));
    }
    Ok(trimmed.to_string())
}

fn extract_langchain(source: &str, path: &Path) -> ArgoResult<String> {
    let triple = Regex::new(r#"(?s)PromptTemplate\s*\([^)]*template\s*=\s*"{3}(.*?)"{3}"#)
        .map_err(|e| ArgoError::Provider(format!("compile langchain regex: {e}")))?;
    if let Some(cap) = triple.captures(source) {
        return Ok(cap[1].to_string());
    }
    let single = Regex::new(r#"(?s)PromptTemplate\s*\([^)]*template\s*=\s*"((?:[^"\\]|\\.)*)""#)
        .map_err(|e| ArgoError::Provider(format!("compile langchain regex: {e}")))?;
    if let Some(cap) = single.captures(source) {
        return Ok(unescape_double_quoted(&cap[1]));
    }
    Err(ArgoError::Schema(format!(
        "no PromptTemplate(template=...) found in {} — try --format raw or --format markdown",
        path.display()
    )))
}

fn unescape_double_quoted(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn extract_markdown(source: &str) -> String {
    if let Some(code) = first_fenced_block(source) {
        return code;
    }
    let mut lines = source.lines();
    let mut title: Option<&str> = None;
    let mut body_start = 0usize;
    let mut consumed = 0usize;
    for line in lines.by_ref() {
        consumed += line.len() + 1;
        if let Some(rest) = line.strip_prefix("# ") {
            title = Some(rest.trim());
            body_start = consumed;
            break;
        }
    }
    let body_after_h1 = source.get(body_start..).unwrap_or("").trim();
    match title {
        Some(t) if !body_after_h1.is_empty() => format!("# {t}\n\n{body_after_h1}"),
        Some(t) => format!("# {t}"),
        None => source.to_string(),
    }
}

fn first_fenced_block(source: &str) -> Option<String> {
    let mut buf: Vec<&str> = Vec::new();
    let mut in_fence = false;
    for line in source.split_inclusive('\n') {
        if line.trim_start().starts_with("```") {
            if in_fence {
                let body = buf.concat();
                return Some(body.trim_end_matches('\n').to_string());
            }
            in_fence = true;
            continue;
        }
        if in_fence {
            buf.push(line);
        }
    }
    None
}

fn write_scaffold(dest: &Path, name: &str, prompt: &str) -> ArgoResult<()> {
    std::fs::write(dest.join("system-prompt.md"), trailing_newline(prompt))?;
    std::fs::write(dest.join("memory-schema.toml"), MEMORY_SCHEMA_STUB)?;
    std::fs::write(dest.join("eval-seed.toml"), EVAL_SEED_STUB)?;
    let manifest = render_manifest(name);
    std::fs::write(dest.join("vertical.toml"), manifest)?;
    Ok(())
}

fn trailing_newline(s: &str) -> String {
    if s.ends_with('\n') {
        s.to_string()
    } else {
        format!("{s}\n")
    }
}

fn render_manifest(name: &str) -> String {
    format!(
        "schema_version = \"1\"\n\
\n\
[vertical]\n\
id = \"{name}\"\n\
name = \"{name}\"\n\
version = \"0.1.0\"\n\
description = \"Imported by `argo genesis import`. Run `argo genesis refine {name}` to flesh out actions/memory/eval.\"\n\
\n\
[role]\n\
system_prompt = \"system-prompt.md\"\n\
\n\
[memory]\n\
schema = \"memory-schema.toml\"\n\
\n\
[evaluation]\n\
suites = [\"eval-seed.toml\"]\n",
    )
}

const MEMORY_SCHEMA_STUB: &str = "# Imported by `argo genesis import`. Memory-schema-builder fills this in.\nschema_version = \"1\"\n";

const EVAL_SEED_STUB: &str = "# Imported by `argo genesis import`. eval-seed-generator fills this in.\nschema_version = \"1\"\n";

fn validate_name(name: &str) -> ArgoResult<()> {
    if name.is_empty() {
        return Err(ArgoError::Schema("--name must not be empty".into()));
    }
    let ok = name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !ok {
        return Err(ArgoError::Schema(format!(
            "--name '{name}' must be lowercase ASCII letters, digits, or '-'"
        )));
    }
    if name.starts_with('-') || name.ends_with('-') {
        return Err(ArgoError::Schema(format!(
            "--name '{name}' must not start or end with '-'"
        )));
    }
    Ok(())
}

#[cfg(test)]
#[path = "genesis_import_tests.rs"]
mod tests;
