//! Small RFC 4180 writer for `argo budget --format csv`.
use super::budget::{BudgetSummary, ChargeRow};
use super::budget_watch::BudgetAlertRow;
use argo_store::spend_anomaly::SpendAnomaly;

#[derive(Debug, Default)]
pub(crate) struct Csv {
    out: String,
}

pub(crate) fn summary(row: &BudgetSummary) -> String {
    let mut csv = Csv::new(&[
        "worker_id",
        "today_usd_micros",
        "this_month_usd_micros",
        "today_usd",
        "this_month_usd",
        "daily_cap_usd_micros",
        "monthly_cap_usd_micros",
        "remaining_today_usd",
        "remaining_month_usd",
        "last_charge_count",
    ]);
    csv.row(&[
        row.worker_id.clone(),
        row.today_usd_micros.to_string(),
        row.this_month_usd_micros.to_string(),
        row.today_usd.to_string(),
        row.this_month_usd.to_string(),
        opt(row.daily_cap_usd_micros),
        opt(row.monthly_cap_usd_micros),
        opt(row.remaining_today_usd),
        opt(row.remaining_month_usd),
        row.last_charges.len().to_string(),
    ]);
    csv.finish()
}

pub(crate) fn charges(rows: &[ChargeRow]) -> String {
    let mut csv = Csv::new(&[
        "worker_id",
        "action_id",
        "usd_micros",
        "usd",
        "charged_at",
        "charged_at_epoch",
    ]);
    for row in rows {
        csv.row(&[
            row.worker_id.clone(),
            row.action_id.clone(),
            row.usd_micros.to_string(),
            row.usd.to_string(),
            row.charged_at.clone(),
            row.charged_at_epoch.to_string(),
        ]);
    }
    csv.finish()
}

pub(crate) fn anomalies(rows: &[SpendAnomaly]) -> String {
    let mut csv = Csv::new(&[
        "id",
        "worker_id",
        "observed_usd_per_hour",
        "baseline_mean_usd",
        "baseline_stddev_usd",
        "z_score",
        "hour_of_day",
        "observed_at_micros",
    ]);
    for row in rows {
        csv.row(&[
            row.id.clone(),
            row.worker_id.clone(),
            row.observed_usd_per_hour.to_string(),
            row.baseline_mean_usd.to_string(),
            row.baseline_stddev_usd.to_string(),
            row.z_score.to_string(),
            row.hour_of_day.to_string(),
            row.observed_at_micros.to_string(),
        ]);
    }
    csv.finish()
}

pub(crate) fn alerts(rows: &[BudgetAlertRow]) -> String {
    let mut csv = Csv::new(&[
        "id",
        "worker_id",
        "threshold",
        "fired_on",
        "fired_at",
        "fired_at_epoch",
        "spend_amount_usd_micros",
        "spend_amount_usd",
        "headroom_amount_usd_micros",
        "headroom_amount_usd",
        "notified_via",
        "synthetic",
    ]);
    for row in rows {
        csv.row(&[
            row.id.clone(),
            row.worker_id.clone(),
            row.threshold.to_string(),
            row.fired_on.clone(),
            row.fired_at.clone(),
            row.fired_at_epoch.to_string(),
            row.spend_amount_usd_micros.to_string(),
            row.spend_amount_usd.to_string(),
            row.headroom_amount_usd_micros.to_string(),
            row.headroom_amount_usd.to_string(),
            row.notified_via.clone(),
            row.synthetic.to_string(),
        ]);
    }
    csv.finish()
}

fn opt<T: ToString>(value: Option<T>) -> String {
    value.map(|v| v.to_string()).unwrap_or_default()
}

impl Csv {
    pub(crate) fn new(headers: &[&str]) -> Self {
        let mut csv = Self::default();
        csv.row(headers);
        csv
    }

    pub(crate) fn row<S: AsRef<str>>(&mut self, fields: &[S]) {
        for (idx, field) in fields.iter().enumerate() {
            if idx > 0 {
                self.out.push(',');
            }
            self.field(field.as_ref());
        }
        self.out.push('\n');
    }

    pub(crate) fn finish(self) -> String {
        self.out
    }

    fn field(&mut self, value: &str) {
        self.out.push('"');
        for ch in value.chars() {
            if ch == '"' {
                self.out.push('"');
            }
            self.out.push(ch);
        }
        self.out.push('"');
    }
}

#[cfg(test)]
mod tests {
    use super::Csv;

    #[test]
    fn escapes_commas_quotes_and_newlines() {
        let mut csv = Csv::new(&["name", "note"]);
        csv.row(&["worker,one", "said \"yes\"\nthen left"]);
        assert_eq!(
            csv.finish(),
            "\"name\",\"note\"\n\"worker,one\",\"said \"\"yes\"\"\nthen left\"\n"
        );
    }

    #[test]
    fn emits_header_only_when_no_rows_follow() {
        let csv = Csv::new(&["id", "amount"]);
        assert_eq!(csv.finish(), "\"id\",\"amount\"\n");
    }

    #[test]
    fn keeps_empty_fields_round_trippable() {
        let mut csv = Csv::new(&["id", "optional"]);
        csv.row(&["row-1", ""]);
        assert_eq!(csv.finish(), "\"id\",\"optional\"\n\"row-1\",\"\"\n");
    }

    #[test]
    fn preserves_numeric_fields_as_csv_fields() {
        let mut csv = Csv::new(&["worker_id", "usd"]);
        csv.row(&["w-1", "1.2500"]);
        assert_eq!(csv.finish(), "\"worker_id\",\"usd\"\n\"w-1\",\"1.2500\"\n");
    }
}
