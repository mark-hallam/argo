//! Cron expression — five-field POSIX-ish syntax:
//! `<minute> <hour> <day-of-month> <month> <day-of-week>`.
//!
//! Each field is one of:
//! - `*` (wildcard)
//! - a literal `N` (integer in field range)
//! - a comma list `N,M,...`
//! - a closed range `A-B`
//!
//! The intersection of all five fields is `match()`-tested against
//! a [`chrono::DateTime<Utc>`]. Step syntax (`*/N`) is reserved for
//! a follow-up; the spec's "every minute"/"every 5 minutes" /
//! "every hour at minute 0" cases are reachable by listing the
//! values explicitly today.
use chrono::{DateTime, Datelike, Timelike, Utc};
use serde::{Deserialize, Serialize};

/// Parse error.
#[derive(Debug, Clone, PartialEq, Eq, ::thiserror::Error)]
pub enum CronParseError {
    /// Wrong number of space-separated fields.
    #[error("expected 5 fields, got {0}")]
    WrongArity(usize),
    /// A field had a bad token.
    #[error("invalid field {field}: {detail}")]
    InvalidField {
        /// Which of the five fields (`minute`, `hour`, ...).
        field: &'static str,
        /// What was wrong.
        detail: String,
    },
    /// Range start > end.
    #[error("invalid range in {field}: {start}-{end}")]
    BadRange {
        /// Which field.
        field: &'static str,
        /// Start.
        start: u32,
        /// End.
        end: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
enum Field {
    Wildcard,
    Values(Vec<u32>),
}

impl Field {
    fn matches(&self, v: u32) -> bool {
        match self {
            Self::Wildcard => true,
            Self::Values(xs) => xs.contains(&v),
        }
    }
}

/// Parsed five-field cron expression.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CronExpr {
    /// Original source text — preserved for serialization round-trips.
    pub source: String,
    minute: Field,
    hour: Field,
    day_of_month: Field,
    month: Field,
    day_of_week: Field,
}

impl CronExpr {
    /// Parse a five-field cron expression.
    ///
    /// # Errors
    /// Returns [`CronParseError`] for the wrong number of fields,
    /// out-of-range values, or malformed range syntax.
    pub fn parse(s: &str) -> Result<Self, CronParseError> {
        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.len() != 5 {
            return Err(CronParseError::WrongArity(parts.len()));
        }
        Ok(Self {
            source: s.to_string(),
            minute: parse_field("minute", parts[0], 0, 59)?,
            hour: parse_field("hour", parts[1], 0, 23)?,
            day_of_month: parse_field("day_of_month", parts[2], 1, 31)?,
            month: parse_field("month", parts[3], 1, 12)?,
            day_of_week: parse_field("day_of_week", parts[4], 0, 6)?,
        })
    }

    /// `true` if `now` matches every field. Cron semantics treat
    /// day-of-month and day-of-week as **independent matches**: if
    /// either field is non-wildcard, the row fires when either
    /// matches (POSIX behavior). If both are wildcards, the date
    /// half always matches.
    #[must_use]
    pub fn matches(&self, now: DateTime<Utc>) -> bool {
        if !self.minute.matches(now.minute()) {
            return false;
        }
        if !self.hour.matches(now.hour()) {
            return false;
        }
        if !self.month.matches(now.month()) {
            return false;
        }
        let dom_wild = matches!(self.day_of_month, Field::Wildcard);
        let dow_wild = matches!(self.day_of_week, Field::Wildcard);
        // chrono uses Monday=0 .. Sunday=6 in num_days_from_monday.
        // POSIX cron uses Sunday=0 .. Saturday=6, with 7=Sunday too.
        // Map to POSIX: Sunday=0, Monday=1, ..., Saturday=6.
        let posix_dow = match now.weekday() {
            chrono::Weekday::Sun => 0,
            chrono::Weekday::Mon => 1,
            chrono::Weekday::Tue => 2,
            chrono::Weekday::Wed => 3,
            chrono::Weekday::Thu => 4,
            chrono::Weekday::Fri => 5,
            chrono::Weekday::Sat => 6,
        };
        let dom_ok = self.day_of_month.matches(now.day());
        let dow_ok = self.day_of_week.matches(posix_dow);
        if dom_wild && dow_wild {
            true
        } else if dom_wild {
            dow_ok
        } else if dow_wild {
            dom_ok
        } else {
            dom_ok || dow_ok
        }
    }
}

fn parse_field(name: &'static str, s: &str, lo: u32, hi: u32) -> Result<Field, CronParseError> {
    if s == "*" {
        return Ok(Field::Wildcard);
    }
    let mut values = Vec::new();
    for tok in s.split(',') {
        if let Some((a, b)) = tok.split_once('-') {
            let start = parse_u32(name, a, lo, hi)?;
            let end = parse_u32(name, b, lo, hi)?;
            if start > end {
                return Err(CronParseError::BadRange {
                    field: name,
                    start,
                    end,
                });
            }
            for v in start..=end {
                values.push(v);
            }
        } else {
            values.push(parse_u32(name, tok, lo, hi)?);
        }
    }
    values.sort_unstable();
    values.dedup();
    Ok(Field::Values(values))
}

fn parse_u32(field: &'static str, s: &str, lo: u32, hi: u32) -> Result<u32, CronParseError> {
    let v: u32 = s.parse().map_err(|_| CronParseError::InvalidField {
        field,
        detail: format!("not a number: {s}"),
    })?;
    if v < lo || v > hi {
        return Err(CronParseError::InvalidField {
            field,
            detail: format!("{v} out of range [{lo},{hi}]"),
        });
    }
    Ok(v)
}
