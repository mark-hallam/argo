use super::*;
use ratatui::backend::TestBackend;
use ratatui::Terminal;

fn render_to_string(monitor: &SpendMonitor) -> String {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| monitor.render(frame, frame.area()))
        .unwrap();
    let buffer = terminal.backend().buffer().clone();
    let mut out = String::new();
    for y in 0..24 {
        for x in 0..80 {
            out.push_str(buffer.cell((x, y)).map(|c| c.symbol()).unwrap_or(""));
        }
        out.push('\n');
    }
    out
}

#[test]
fn render_empty_spend_monitor() {
    let monitor = SpendMonitor::from_global_spend(GlobalSpend::default());
    let frame = render_to_string(&monitor);
    assert!(frame.contains("spend monitor"));
    assert!(frame.contains("today by hour"));
    assert!(frame.contains("$0.00"));
    assert!(frame.contains("budget: not set"));
}

#[test]
fn render_populated_today_week_month() {
    let monitor = SpendMonitor::from_global_spend(GlobalSpend {
        today: 1_500_000,
        this_month: 12_000_000,
        all_time: 25_000_000,
    });
    let frame = render_to_string(&monitor);
    assert!(frame.contains("today by hour"));
    assert!(frame.contains("week by day"));
    assert!(frame.contains("month by day"));
    assert!(frame.contains("$1.50"));
    assert!(frame.contains("$12.00"));
}

#[test]
fn series_uses_fixed_point_counts_and_keeps_outlier() {
    let series = SpendSeries::from_total("today by hour", 24, 99_000_000);
    assert_eq!(series.points.len(), 24);
    assert_eq!(series.points.last(), Some(&99_000_000));
    assert_eq!(
        series.points.iter().filter(|point| **point == 0).count(),
        23
    );
}

#[test]
fn budget_bands_follow_thresholds() {
    assert_eq!(BudgetSnapshot::band(49, Some(100)), BudgetBand::Green);
    assert_eq!(BudgetSnapshot::band(50, Some(100)), BudgetBand::Amber);
    assert_eq!(BudgetSnapshot::band(80, Some(100)), BudgetBand::Amber);
    assert_eq!(BudgetSnapshot::band(81, Some(100)), BudgetBand::Red);
    assert_eq!(BudgetSnapshot::band(10, None), BudgetBand::Unset);
}

#[test]
fn r_key_requests_refresh() {
    let monitor = SpendMonitor::from_global_spend(GlobalSpend::default());
    assert_eq!(
        monitor.handle_key(KeyCode::Char('r')),
        SpendOutcome::Refresh
    );
}

#[test]
fn q_and_escape_close_overlay() {
    let monitor = SpendMonitor::from_global_spend(GlobalSpend::default());
    assert_eq!(monitor.handle_key(KeyCode::Char('q')), SpendOutcome::Close);
    assert_eq!(monitor.handle_key(KeyCode::Esc), SpendOutcome::Close);
}
