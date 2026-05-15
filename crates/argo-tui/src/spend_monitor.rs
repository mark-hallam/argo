//! Fleet spend monitor overlay with compact sparkline summaries.
use crate::app::ViewerState;
use argo_store::{GlobalSpend, SessionStore};
use argo_types::ArgoResult;
use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Sparkline};
use ratatui::Frame;
use std::sync::Arc;

/// One spend series rendered as a sparkline.
#[derive(Clone, Debug, PartialEq)]
pub struct SpendSeries {
    /// Human label for the series.
    pub label: &'static str,
    /// USD-micro points rendered by ratatui's sparkline widget.
    pub points: Vec<u64>,
    /// Total spend represented by the series, in USD micros.
    pub total: u64,
}

/// Optional budget ceilings used for headroom labels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BudgetSnapshot {
    /// Optional daily spend ceiling, in USD micros.
    pub today_limit: Option<u64>,
    /// Optional weekly spend ceiling, in USD micros.
    pub week_limit: Option<u64>,
    /// Optional monthly spend ceiling, in USD micros.
    pub month_limit: Option<u64>,
}

/// Utilization band for the budget headroom line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BudgetBand {
    /// No ceiling is configured.
    Unset,
    /// Less than 50% of the ceiling is spent.
    Green,
    /// 50-80% of the ceiling is spent.
    Amber,
    /// More than 80% of the ceiling is spent.
    Red,
}

/// Result of handling a key while the spend monitor is open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpendOutcome {
    /// Close the overlay.
    Close,
    /// Refresh the spend snapshot from the store.
    Refresh,
    /// Consume the key without changing state.
    None,
}

/// Full spend monitor snapshot rendered by the overlay.
#[derive(Clone, Debug, PartialEq)]
pub struct SpendMonitor {
    /// Today by hour.
    pub today: SpendSeries,
    /// Current week by day.
    pub week: SpendSeries,
    /// Current month by day.
    pub month: SpendSeries,
    /// Optional budget ceilings.
    pub budget: BudgetSnapshot,
}

impl SpendMonitor {
    /// Build a monitor snapshot from the existing global spend rollup.
    #[must_use]
    pub fn from_global_spend(spend: GlobalSpend) -> Self {
        Self {
            today: SpendSeries::from_total("today by hour", 24, spend.today),
            week: SpendSeries::from_total("week by day", 7, spend.today),
            month: SpendSeries::from_total("month by day", 30, spend.this_month),
            budget: BudgetSnapshot::default(),
        }
    }

    /// Refresh the monitor from the store's existing spend rollup.
    pub async fn refresh_from_store(&mut self, store: &SessionStore) -> ArgoResult<()> {
        let refreshed = Self::from_global_spend(store.load_global_spend().await?);
        self.today = refreshed.today;
        self.week = refreshed.week;
        self.month = refreshed.month;
        Ok(())
    }

    /// Handle a key while the overlay is focused.
    #[must_use]
    pub fn handle_key(&self, key: KeyCode) -> SpendOutcome {
        match key {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('$') => SpendOutcome::Close,
            KeyCode::Char('r') => SpendOutcome::Refresh,
            _ => SpendOutcome::None,
        }
    }

    /// Render the monitor overlay.
    pub fn render(&self, frame: &mut Frame<'_>, area: Rect) {
        let w = area.width.saturating_mul(9) / 10;
        let h = area.height.saturating_mul(8) / 10;
        let x = area.x + (area.width.saturating_sub(w)) / 2;
        let y = area.y + (area.height.saturating_sub(h)) / 2;
        let panel = Rect::new(x, y, w.max(54), h.max(16));
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" spend monitor — r refresh · q/Esc/$ closes ");
        let inner = block.inner(panel);
        frame.render_widget(Clear, panel);
        frame.render_widget(block, panel);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4),
                Constraint::Length(4),
                Constraint::Length(4),
                Constraint::Min(1),
            ])
            .split(inner);
        render_series(frame, rows[0], &self.today, self.budget.today_limit);
        render_series(frame, rows[1], &self.week, self.budget.week_limit);
        render_series(frame, rows[2], &self.month, self.budget.month_limit);
    }
}

impl SpendSeries {
    /// Build a fixed-width series from the available aggregate total.
    #[must_use]
    pub fn from_total(label: &'static str, len: usize, total: u64) -> Self {
        let mut points = vec![0; len.max(1)];
        if let Some(last) = points.last_mut() {
            *last = total;
        }
        Self {
            label,
            points,
            total,
        }
    }
}

impl BudgetSnapshot {
    /// Return the utilization band for `spent` against `limit`.
    #[must_use]
    pub fn band(spent: u64, limit: Option<u64>) -> BudgetBand {
        let Some(limit) = limit.filter(|limit| *limit > 0) else {
            return BudgetBand::Unset;
        };
        let pct = spent as f64 / limit as f64;
        if pct < 0.5 {
            BudgetBand::Green
        } else if pct <= 0.8 {
            BudgetBand::Amber
        } else {
            BudgetBand::Red
        }
    }
}

/// Open the spend monitor from the current store snapshot.
pub(crate) async fn open_spend_monitor(
    state: &mut ViewerState,
    store: &Arc<SessionStore>,
) -> ArgoResult<()> {
    let spend = store.load_global_spend().await.unwrap_or_default();
    state.spend_monitor = Some(SpendMonitor::from_global_spend(spend));
    Ok(())
}

/// Handle keys while the spend monitor is open.
pub(crate) async fn handle_spend_monitor_key(
    state: &mut ViewerState,
    store: &Arc<SessionStore>,
    key: KeyCode,
) -> ArgoResult<bool> {
    let Some(monitor) = state.spend_monitor.as_mut() else {
        return Ok(false);
    };
    match monitor.handle_key(key) {
        SpendOutcome::Close => state.spend_monitor = None,
        SpendOutcome::Refresh => {
            monitor.refresh_from_store(store).await?;
        }
        SpendOutcome::None => {}
    }
    Ok(true)
}

fn render_series(frame: &mut Frame<'_>, area: Rect, series: &SpendSeries, limit: Option<u64>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(area);
    let band = BudgetSnapshot::band(series.total, limit);
    let (label, color) = budget_label(series.total, limit, band);
    let header = Line::from(vec![
        Span::styled(series.label, Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(format!("  total {}", usd(series.total))),
        Span::raw("  "),
        Span::styled(label, Style::default().fg(color)),
    ]);
    frame.render_widget(Paragraph::new(header), chunks[0]);
    frame.render_widget(
        Sparkline::default()
            .block(Block::default().borders(Borders::BOTTOM))
            .data(&series.points)
            .style(Style::default().fg(Color::Cyan)),
        chunks[1],
    );
}

fn budget_label(spent: u64, limit: Option<u64>, band: BudgetBand) -> (String, Color) {
    match (limit, band) {
        (Some(limit), BudgetBand::Green) => (
            format!("budget {} / {} (<50%)", usd(spent), usd(limit)),
            Color::Green,
        ),
        (Some(limit), BudgetBand::Amber) => (
            format!("budget {} / {} (50-80%)", usd(spent), usd(limit)),
            Color::Yellow,
        ),
        (Some(limit), BudgetBand::Red) => (
            format!("budget {} / {} (>80%)", usd(spent), usd(limit)),
            Color::Red,
        ),
        _ => ("budget: not set".to_string(), Color::DarkGray),
    }
}

fn usd(micros: u64) -> String {
    format!("${:.2}", micros as f64 / 1_000_000.0)
}

#[cfg(test)]
#[path = "spend_monitor_tests.rs"]
mod spend_monitor_tests;
