//! Slides backend — markdown-with-`---`-separators → reveal.js
//! HTML.
//!
//! Reveal.js is loaded via a CDN-free embedded version in v1: a
//! tiny CSS + JS stub that handles arrow-key navigation between
//! `<section>` slides. We deliberately do NOT pull the full
//! reveal.js bundle (1.3 MB) — the operator can swap the
//! `--reveal-bundle` env override in v1.x for the full library if
//! they want themes / speaker notes / fragments.
//!
//! Slide separator: `\n---\n` on its own line. Empty leading /
//! trailing slides are dropped so a markdown body that starts
//! with `---` doesn't ship a blank first slide.

use crate::error::DocResult;
use std::path::Path;

const SLIDE_SEPARATOR: &str = "\n---\n";

/// Split markdown into per-slide chunks. Strips empty chunks
/// (so leading / trailing `---` don't materialise blank slides).
/// A leading `---\n` (without the prefix `\n` the main separator
/// requires) is also tolerated and dropped.
pub fn split_slides(markdown: &str) -> Vec<String> {
    let trimmed = markdown
        .strip_prefix("---\n")
        .or_else(|| markdown.strip_prefix("---\r\n"))
        .unwrap_or(markdown);
    trimmed
        .split(SLIDE_SEPARATOR)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

/// Render the full reveal.js HTML for the given markdown source +
/// title. The output is self-contained — no external CDN fetch
/// needed to navigate the deck.
pub fn render_slides_html(markdown: &str, title: &str) -> String {
    let safe_title = html_escape(title);
    let slides = split_slides(markdown);
    let mut sections = String::new();
    for slide_md in &slides {
        let mut html_body = String::with_capacity(slide_md.len() * 2);
        let parser = pulldown_cmark::Parser::new(slide_md);
        pulldown_cmark::html::push_html(&mut html_body, parser);
        sections.push_str("<section>\n");
        sections.push_str(&html_body);
        sections.push_str("</section>\n");
    }
    format!(
        "<!doctype html>
<html><head><meta charset=\"utf-8\"><title>{safe_title}</title>
<style>
:root {{ color-scheme: light dark; }}
html, body {{ margin: 0; padding: 0; height: 100%; }}
body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; background: #1a1a1a; color: #fafafa; }}
.deck {{ height: 100vh; display: flex; align-items: center; justify-content: center; }}
section {{ display: none; max-width: 900px; padding: 40px; line-height: 1.4; font-size: 24px; }}
section.active {{ display: block; }}
section h1 {{ font-size: 48px; margin-top: 0; }}
section h2 {{ font-size: 36px; }}
section h3 {{ font-size: 28px; }}
section pre {{ background: #2a2a2a; padding: 12px; border-radius: 6px; font-family: 'SF Mono', Menlo, Consolas, monospace; font-size: 18px; overflow-x: auto; }}
section code {{ font-family: 'SF Mono', Menlo, Consolas, monospace; background: #2a2a2a; padding: 2px 6px; border-radius: 3px; }}
section blockquote {{ border-left: 4px solid #555; margin: 12px 0; padding-left: 16px; color: #ccc; }}
.nav {{ position: fixed; bottom: 16px; right: 24px; opacity: 0.6; font-size: 14px; }}
</style></head><body>
<div class=\"deck\" id=\"deck\">
{sections}</div>
<div class=\"nav\">← / → to navigate · <span id=\"counter\"></span></div>
<script>
(function() {{
  const slides = document.querySelectorAll('#deck section');
  if (slides.length === 0) return;
  let i = 0;
  const counter = document.getElementById('counter');
  function show() {{
    slides.forEach((s, idx) => s.classList.toggle('active', idx === i));
    counter.textContent = (i + 1) + ' / ' + slides.length;
  }}
  document.addEventListener('keydown', (e) => {{
    if (e.key === 'ArrowRight' || e.key === ' ') {{
      i = Math.min(i + 1, slides.length - 1);
      show();
    }} else if (e.key === 'ArrowLeft') {{
      i = Math.max(i - 1, 0);
      show();
    }} else if (e.key === 'Home') {{
      i = 0; show();
    }} else if (e.key === 'End') {{
      i = slides.length - 1; show();
    }}
  }});
  show();
}})();
</script>
</body></html>"
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Render + write to disk. Returns the byte count.
pub fn write_slides(target: &Path, markdown: &str, title: &str) -> DocResult<u64> {
    let html = render_slides_html(markdown, title);
    if let Some(parent) = target.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| {
                crate::error::DocExecutorError::io(
                    format!("slides: create parent {}", parent.display()),
                    e,
                )
            })?;
        }
    }
    std::fs::write(target, html.as_bytes()).map_err(|e| {
        crate::error::DocExecutorError::io(
            format!("slides: write {}", target.display()),
            e,
        )
    })?;
    Ok(html.len() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_slides_segments_on_triple_dash() {
        let md = "# Slide 1\n\nintro\n---\n# Slide 2\n\nmore";
        let slides = split_slides(md);
        assert_eq!(slides.len(), 2);
        assert!(slides[0].contains("Slide 1"));
        assert!(slides[1].contains("Slide 2"));
    }

    #[test]
    fn split_slides_drops_leading_empty_chunk() {
        let md = "---\n# First slide\n\nbody\n---\n# Second";
        let slides = split_slides(md);
        assert_eq!(slides.len(), 2);
        assert!(slides[0].starts_with("# First"));
    }

    #[test]
    fn render_slides_html_emits_section_per_slide() {
        let md = "# A\n---\n# B\n---\n# C";
        let html = render_slides_html(md, "Deck");
        assert_eq!(html.matches("<section>").count(), 3);
        assert!(html.contains("<title>Deck</title>"));
        assert!(html.contains("<h1>A</h1>"));
        assert!(html.contains("<h1>B</h1>"));
        assert!(html.contains("<h1>C</h1>"));
    }

    #[test]
    fn render_slides_html_includes_keyboard_nav_script() {
        let html = render_slides_html("# Solo", "T");
        assert!(html.contains("ArrowRight"));
        assert!(html.contains("ArrowLeft"));
    }
}
