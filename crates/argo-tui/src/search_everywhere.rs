//! Self-contained Ctrl-K search modal for sessions, messages, audit, and skills.
use crate::app::{PaneFocus, ViewerState};
use crate::viewer::{fetch_message_count, fetch_messages};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use std::{
    io::Stdout,
    sync::Arc,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum ScopeFilter {
    #[default]
    All,
    Sessions,
    Messages,
    Audit,
    Skills,
}

impl ScopeFilter {
    fn next(self) -> Self {
        [
            Self::All,
            Self::Sessions,
            Self::Messages,
            Self::Audit,
            Self::Skills,
        ][(self as usize + 1) % 5]
    }

    fn label(self) -> &'static str {
        ["All", "Sessions", "Messages", "Audit", "Skills"][self as usize]
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum SearchResult {
    Session(SessionId, String, usize),
    Message(SessionId, i64, String, usize),
    Audit(SessionId, i64, String, usize),
    Skill(String, String, usize),
}

impl SearchResult {
    fn score(&self) -> usize {
        match self {
            Self::Session(_, _, score)
            | Self::Message(_, _, _, score)
            | Self::Audit(_, _, _, score)
            | Self::Skill(_, _, score) => *score,
        }
    }

    fn row(&self) -> String {
        match self {
            Self::Session(id, title, _) => format!("session {title} · {id}"),
            Self::Message(_, seq, snippet, _) => format!("message #{seq} · {snippet}"),
            Self::Audit(_, seq, summary, _) => format!("audit #{seq} · {summary}"),
            Self::Skill(name, desc, _) => format!("skill {name} · {desc}"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum SearchOutcome {
    Stay,
    Close,
    Navigate(SearchResult),
}

#[derive(Debug, Default)]
struct SearchOverlay {
    query: String,
    results: Vec<SearchResult>,
    selected: usize,
    scope: ScopeFilter,
}

#[rustfmt::skip]
impl SearchOverlay {
    fn handle_key(&mut self, key: KeyEvent) -> SearchOutcome {
        match key.code {
            KeyCode::Esc => SearchOutcome::Close,
            KeyCode::Enter => self.results.get(self.selected).cloned().map(SearchOutcome::Navigate).unwrap_or(SearchOutcome::Stay),
            KeyCode::Tab => { self.scope = self.scope.next(); self.selected = 0; SearchOutcome::Stay }
            KeyCode::Backspace => { self.query.pop(); self.selected = 0; SearchOutcome::Stay }
            KeyCode::Down | KeyCode::Char('j') => { self.bump(1); SearchOutcome::Stay }
            KeyCode::Up | KeyCode::Char('k') => { self.bump(self.results.len().saturating_sub(1)); SearchOutcome::Stay }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => { self.query.push(c); self.selected = 0; SearchOutcome::Stay }
            _ => SearchOutcome::Stay,
        }
    }

    fn bump(&mut self, delta: usize) {
        if !self.results.is_empty() {
            self.selected = (self.selected + delta) % self.results.len();
        }
    }

    async fn refresh(&mut self, store: &Arc<SessionStore>, state: &ViewerState) {
        let q = self.query.trim();
        self.results.clear();
        if q.is_empty() {
            return;
        }
        if matches!(self.scope, ScopeFilter::All | ScopeFilter::Sessions) {
            for s in &state.sessions {
                let title = s.title.clone().unwrap_or_else(|| "(untitled)".into());
                if let Some(score) = score_text(&format!("{title} {} {:?}", s.id, s.worker_id), q) {
                    self.results.push(SearchResult::Session(s.id, title, score));
                }
            }
        }
        if matches!(self.scope, ScopeFilter::All | ScopeFilter::Messages) {
            for hit in store.search_messages(q, 20).await.unwrap_or_default() {
                self.results.push(SearchResult::Message(hit.session_id, hit.seq, hit.snippet, 900));
            }
        }
        if matches!(self.scope, ScopeFilter::All | ScopeFilter::Audit) {
            for s in state.sessions.iter().take(50) {
                for row in store.list_audit_for_session(s.id).await.unwrap_or_default() {
                    let summary = format!("{} {}", row.kind, row.payload_json);
                    if let Some(score) = score_text(&summary, q) {
                        self.results.push(SearchResult::Audit(s.id, row.seq, short(&summary), score));
                    }
                }
            }
        }
        if matches!(self.scope, ScopeFilter::All | ScopeFilter::Skills) {
            for skill in argo_skill::SkillStore::new(Arc::clone(store)).list_active().await.unwrap_or_default() {
                let (name, desc) = (skill.frontmatter.name, skill.frontmatter.description);
                if let Some(score) = score_text(&format!("{name} {desc}"), q) {
                    self.results.push(SearchResult::Skill(name, desc, score));
                }
            }
        }
        self.results.sort_by_key(|r| std::cmp::Reverse(r.score()));
        self.selected = self.selected.min(self.results.len().saturating_sub(1));
    }
}

pub(crate) async fn run_search_overlay(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    state: &mut ViewerState,
    store: &Arc<SessionStore>,
) -> ArgoResult<()> {
    let mut overlay = SearchOverlay::default();
    let mut refresh_at: Option<Instant> = None;
    loop {
        if refresh_at.is_some_and(|when| Instant::now() >= when) {
            overlay.refresh(store, state).await;
            refresh_at = None;
        }
        terminal
            .draw(|frame| {
                crate::render::draw(frame, state);
                draw_search(frame, frame.area(), &overlay);
            })
            .map_err(|e| ArgoError::Provider(format!("search draw: {e}")))?;
        if !event::poll(Duration::from_millis(50))
            .map_err(|e| ArgoError::Provider(format!("search poll: {e}")))?
        {
            continue;
        }
        let Event::Key(key) =
            event::read().map_err(|e| ArgoError::Provider(format!("search read: {e}")))?
        else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        let before = (overlay.query.clone(), overlay.scope);
        match overlay.handle_key(key) {
            SearchOutcome::Close => return Ok(()),
            SearchOutcome::Navigate(result) => return apply_result(state, store, result).await,
            SearchOutcome::Stay if before != (overlay.query.clone(), overlay.scope) => {
                refresh_at = Some(Instant::now() + Duration::from_millis(150));
            }
            SearchOutcome::Stay => {}
        }
    }
}

#[rustfmt::skip]
fn draw_search(frame: &mut Frame<'_>, area: Rect, overlay: &SearchOverlay) {
    let panel = Rect::new(
        area.x + area.width.saturating_mul(15) / 100,
        area.y + area.height.saturating_mul(15) / 100,
        (area.width.saturating_mul(7) / 10).max(48),
        (area.height.saturating_mul(7) / 10).max(10),
    );
    let block = Block::default().borders(Borders::ALL).title(format!(" search {} · Tab scope · Enter open · Esc close ", overlay.scope.label()));
    let inner = block.inner(panel);
    frame.render_widget(Clear, panel);
    frame.render_widget(block, panel);
    let rows = Layout::default().direction(Direction::Vertical).constraints([Constraint::Length(3), Constraint::Min(1)]).split(inner);
    frame.render_widget(Paragraph::new(format!("> {}", overlay.query)).block(Block::default().borders(Borders::BOTTOM)), rows[0]);
    let items: Vec<ListItem> = if overlay.results.is_empty() {
        vec![ListItem::new(if overlay.query.is_empty() {
            "Type to search sessions, messages, audit, and skills"
        } else {
            "No results"
        })]
    } else {
        overlay.results.iter().map(|r| ListItem::new(r.row())).collect()
    };
    let mut list_state = ListState::default();
    if !overlay.results.is_empty() {
        list_state.select(Some(overlay.selected));
    }
    let list = List::new(items).highlight_style(Style::default().add_modifier(Modifier::REVERSED)).highlight_symbol(">");
    frame.render_stateful_widget(list, rows[1], &mut list_state);
}

#[rustfmt::skip]
async fn apply_result(
    state: &mut ViewerState,
    store: &Arc<SessionStore>,
    result: SearchResult,
) -> ArgoResult<()> {
    match result {
        SearchResult::Session(id, _, _) => select_session(state, store, id).await,
        SearchResult::Message(id, seq, _, _) => {
            select_session(state, store, id).await?;
            state.focus = PaneFocus::Messages;
            if let Some(idx) = state.selected_messages.iter().position(|m| m.seq == seq) {
                state.messages_scroll = idx;
            }
            Ok(())
        }
        SearchResult::Audit(id, seq, _, _) => {
            select_session(state, store, id).await?;
            let rows = store.list_audit_for_session(id).await.unwrap_or_default();
            state.audit_overlay_scroll = rows.iter().position(|r| r.seq == seq).unwrap_or(0);
            state.audit_overlay = Some(rows);
            Ok(())
        }
        SearchResult::Skill(name, _, _) => {
            let rows = argo_skill::SkillStore::new(Arc::clone(store)).list_active().await.unwrap_or_default();
            state.skills_overlay_scroll = rows.iter().position(|s| s.frontmatter.name == name).unwrap_or(0);
            state.skills_overlay = Some(rows);
            Ok(())
        }
    }
}

async fn select_session(
    state: &mut ViewerState,
    store: &Arc<SessionStore>,
    id: SessionId,
) -> ArgoResult<()> {
    if let Some(idx) = state.sessions.iter().position(|s| s.id == id) {
        state.selected = idx;
        state.selected_message_count = None;
        state.messages_loaded = false;
        state.messages_scroll = 0;
        fetch_message_count(store, state).await?;
        fetch_messages(store, state).await?;
    }
    Ok(())
}

fn score_text(haystack: &str, needle: &str) -> Option<usize> {
    haystack
        .to_ascii_lowercase()
        .find(&needle.to_ascii_lowercase())
        .map(|idx| 1000usize.saturating_sub(idx))
}

fn short(s: &str) -> String {
    if s.len() <= 96 {
        s.to_string()
    } else {
        format!("{}...", &s[..96])
    }
}

#[cfg(test)]
#[path = "search_everywhere_tests.rs"]
mod search_everywhere_tests;
