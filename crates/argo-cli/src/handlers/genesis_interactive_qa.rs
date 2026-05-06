//! Pure Q&A prompt loop for `argo genesis interactive` (Stream
//! `claude-genesis-charter-conversation`). Lives in its own
//! module so the prompt logic is testable without spawning a
//! subprocess: tests inject a `MockReader` with canned answers
//! and a `Vec<u8>` writer, then assert the captured
//! [`CharterDialogue`].
//!
//! The `argo genesis interactive` cli wrapper in
//! [`super::genesis_interactive`] uses [`RealReader`] over
//! `std::io::Stdin`.
use argo_genesis::{CharterDialogue, Question, QUESTIONS};
use argo_types::{ArgoError, ArgoResult};
use std::io::{BufRead, Write};

/// Abstraction over the input source so the prompt loop is
/// testable. `read_line` returns `None` at EOF.
pub(crate) trait LineReader {
    fn read_line(&mut self) -> ArgoResult<Option<String>>;
}

/// Stdin-backed reader used at runtime.
pub(crate) struct RealReader<R: BufRead> {
    inner: R,
}

impl<R: BufRead> RealReader<R> {
    pub(crate) fn new(inner: R) -> Self {
        Self { inner }
    }
}

impl<R: BufRead> LineReader for RealReader<R> {
    fn read_line(&mut self) -> ArgoResult<Option<String>> {
        let mut buf = String::new();
        let n = self
            .inner
            .read_line(&mut buf)
            .map_err(|e| ArgoError::Provider(format!("genesis interactive: stdin: {e}")))?;
        if n == 0 {
            return Ok(None);
        }
        // Strip trailing newline (and Windows-style \r\n).
        while matches!(buf.chars().last(), Some('\n') | Some('\r')) {
            buf.pop();
        }
        Ok(Some(buf))
    }
}

/// Drive the [`QUESTIONS`] table against a [`LineReader`] +
/// [`Write`] and return the assembled [`CharterDialogue`]. Single
/// loop, no recursion; each question dispatches by `key` to the
/// right field on the dialogue under construction.
pub(crate) fn prompt_dialogue<R: LineReader, W: Write>(
    reader: &mut R,
    writer: &mut W,
) -> ArgoResult<CharterDialogue> {
    let mut dialogue = CharterDialogue {
        role: String::new(),
        name: String::new(),
        industry: String::new(),
        duties: Vec::new(),
        channels: Vec::new(),
        spend_cap_per_day_usd: 0.0,
    };
    writeln!(
        writer,
        "argo genesis interactive — answer 6 prompts then a vertical lands at the path you pass via --output."
    )
    .map_err(|e| ArgoError::Provider(format!("genesis interactive: {e}")))?;
    for q in QUESTIONS {
        prompt_one(q, reader, writer, &mut dialogue)?;
    }
    Ok(dialogue)
}

fn prompt_one<R: LineReader, W: Write>(
    q: &Question,
    reader: &mut R,
    writer: &mut W,
    dialogue: &mut CharterDialogue,
) -> ArgoResult<()> {
    if q.multi {
        writeln!(writer, "{prompt}", prompt = q.prompt)
            .map_err(|e| ArgoError::Provider(format!("genesis interactive: {e}")))?;
        let lines = read_until_blank(reader)?;
        if q.required && lines.iter().all(|l| l.trim().is_empty()) {
            return Err(ArgoError::Provider(format!(
                "genesis interactive: '{}' is required (provide at least one line)",
                q.key
            )));
        }
        apply_answer(q.key, &lines.join("\n"), &lines, dialogue)?;
        return Ok(());
    }
    loop {
        write!(writer, "{prompt} > ", prompt = q.prompt)
            .map_err(|e| ArgoError::Provider(format!("genesis interactive: {e}")))?;
        writer
            .flush()
            .map_err(|e| ArgoError::Provider(format!("genesis interactive: {e}")))?;
        let answer = match reader.read_line()? {
            Some(s) => s,
            None => {
                if q.required {
                    return Err(ArgoError::Provider(format!(
                        "genesis interactive: stdin closed before required '{}' answered",
                        q.key
                    )));
                }
                String::new()
            }
        };
        if q.required && answer.trim().is_empty() {
            writeln!(writer, "(required — please answer)")
                .map_err(|e| ArgoError::Provider(format!("genesis interactive: {e}")))?;
            continue;
        }
        apply_answer(q.key, &answer, std::slice::from_ref(&answer), dialogue)?;
        return Ok(());
    }
}

fn read_until_blank<R: LineReader>(reader: &mut R) -> ArgoResult<Vec<String>> {
    let mut out = Vec::new();
    loop {
        match reader.read_line()? {
            None => return Ok(out),
            Some(s) if s.trim().is_empty() => return Ok(out),
            Some(s) => out.push(s),
        }
    }
}

/// Map a captured answer to the corresponding field on
/// [`CharterDialogue`]. Pure function — no IO — so tests can
/// exercise field-mapping in isolation.
pub(crate) fn apply_answer(
    key: &str,
    raw: &str,
    multi_lines: &[String],
    dialogue: &mut CharterDialogue,
) -> ArgoResult<()> {
    match key {
        "role" => {
            dialogue.role = raw.trim().to_string();
        }
        "name" => {
            dialogue.name = raw.trim().to_string();
        }
        "industry" => {
            dialogue.industry = raw.trim().to_string();
        }
        "duties" => {
            dialogue.duties = multi_lines
                .iter()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
        "channels" => {
            dialogue.channels = raw
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect();
        }
        "spend_cap" => {
            let parsed: f64 = raw
                .trim()
                .trim_start_matches('$')
                .parse()
                .map_err(|e| ArgoError::Provider(format!(
                    "genesis interactive: spend cap '{raw}' is not a number: {e}"
                )))?;
            dialogue.spend_cap_per_day_usd = parsed;
        }
        other => {
            return Err(ArgoError::Provider(format!(
                "genesis interactive: unknown question key '{other}'"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "genesis_interactive_qa_tests.rs"]
mod tests;
