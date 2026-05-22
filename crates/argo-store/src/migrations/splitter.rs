//! Comment-aware SQL statement splitter shared by every schema
//! runner in `argo-store`.
//!
//! Replaces naive `&str::split_terminator(';')` which mis-tokenises
//! migrations whose `-- ...` comment lines contain literal `;`
//! characters (e.g. `-- catalog by skill_id; operator-driven`).
//! PR #1988 introduced that pattern across v43/v47/v51 SQL and the
//! gateway refused to boot at migration v43 on every fresh install.
//!
//! The helper is line-based, not a SQL parser: each line is scanned
//! for `--`; anything from the first `--` to end-of-line is stripped;
//! the stripped lines are rejoined with `\n` and split on `;`.
//!
//! Limitation: `--` inside a string literal (e.g. `'foo--bar'`) is
//! technically not a comment per the SQL spec. The pragmatic guard
//! here skips the strip if the line already contains a `'` before
//! the `--`. None of the workspace's `_UP_SQL` constants currently
//! rely on `--` inside string literals.

/// Split `sql` into individual statements, stripping `-- ...`
/// line comments before tokenising on `;`. Returns owned `String`s
/// because comment stripping derives content rather than borrowing
/// from the input.
///
/// # Example
///
/// ```ignore
/// let stmts = split_sql_statements(
///     "-- catalog by skill_id; operator-driven\nCREATE TABLE t (id INTEGER);",
/// );
/// assert_eq!(stmts, vec!["CREATE TABLE t (id INTEGER)"]);
/// ```
pub(crate) fn split_sql_statements(sql: &str) -> Vec<String> {
    let stripped = sql
        .lines()
        .map(strip_line_comment)
        .collect::<Vec<_>>()
        .join("\n");

    stripped
        .split(';')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Strip `-- ...` from the first occurrence of `--` to end-of-line.
/// If a single quote (`'`) precedes the `--`, treat the `--` as
/// potentially inside a string literal and leave the line untouched.
fn strip_line_comment(line: &str) -> &str {
    let Some(comment_at) = line.find("--") else {
        return line;
    };
    let before = &line[..comment_at];
    if before.contains('\'') {
        return line;
    }
    before
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semicolon_in_comment_is_ignored() {
        let sql = "-- semicolon ; in comment\nCREATE TABLE t (id INTEGER);";
        assert_eq!(
            split_sql_statements(sql),
            vec!["CREATE TABLE t (id INTEGER)"],
        );
    }

    #[test]
    fn multi_line_comment_block_with_semicolons() {
        let sql = "-- line one\n-- line two with ;\nSELECT 1;";
        assert_eq!(split_sql_statements(sql), vec!["SELECT 1"]);
    }

    #[test]
    fn trailing_semicolon() {
        let sql = "SELECT 1;";
        assert_eq!(split_sql_statements(sql), vec!["SELECT 1"]);
    }

    #[test]
    fn leading_semicolon() {
        let sql = ";SELECT 1;";
        assert_eq!(split_sql_statements(sql), vec!["SELECT 1"]);
    }

    #[test]
    fn empty_input() {
        assert!(split_sql_statements("").is_empty());
    }

    #[test]
    fn comment_only_input() {
        let sql = "-- nothing but a comment ; with a semicolon\n-- and another";
        assert!(split_sql_statements(sql).is_empty());
    }

    #[test]
    fn whitespace_only_input() {
        assert!(split_sql_statements("   \n\n  \t\n").is_empty());
    }

    #[test]
    fn multiple_statements() {
        let sql = "CREATE TABLE a (id INTEGER);\nCREATE INDEX i ON a(id);";
        assert_eq!(
            split_sql_statements(sql),
            vec!["CREATE TABLE a (id INTEGER)", "CREATE INDEX i ON a(id)"],
        );
    }

    #[test]
    fn inline_comment_after_statement() {
        let sql = "CREATE TABLE t (id INTEGER); -- trailing ; note\nSELECT 1;";
        assert_eq!(
            split_sql_statements(sql),
            vec!["CREATE TABLE t (id INTEGER)", "SELECT 1"],
        );
    }

    #[test]
    fn dash_inside_string_literal_not_stripped() {
        let sql = "INSERT INTO t (name) VALUES ('foo--bar');";
        assert_eq!(
            split_sql_statements(sql),
            vec!["INSERT INTO t (name) VALUES ('foo--bar')"],
        );
    }

    /// Regression test pinned to the exact comment text introduced by
    /// PR #1988 in `migrations/47__skill_canary.sql` line 6. Without
    /// the comment-aware split, SQLite saw `-- catalog by `skill_id`
    /// then choked on ` operator-driven and not tenant-partitioned.`
    /// as a syntax error and the gateway refused to boot at v43.
    #[test]
    fn regression_pr_1988_skill_canary_comment() {
        let sql = "-- SAFETY: global table because skills are a cross-tenant marketplace\n\
                   -- catalog (see `skill` in v1.rs). Canary rollouts target the global\n\
                   -- catalog by `skill_id`; operator-driven and not tenant-partitioned.\n\
                   CREATE TABLE IF NOT EXISTS skill_canary (\n\
                       skill_id          TEXT PRIMARY KEY\n\
                   );";
        let stmts = split_sql_statements(sql);
        assert_eq!(stmts.len(), 1);
        assert!(stmts[0].starts_with("CREATE TABLE IF NOT EXISTS skill_canary"));
    }
}
