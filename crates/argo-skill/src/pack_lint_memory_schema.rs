//! Memory-schema JSON validation for `crate::pack_lint`.

use std::collections::HashSet;
use std::path::Path;

use serde_json::{Map, Value};

use super::pack_lint::{push, CheckId, LintFinding, PackLintSeverity};

const SUPPORTED_TABLE_COLUMN_TYPES: &[&str] =
    &["string", "int", "float", "bool", "json", "datetime"];

pub(super) fn check_json(path: &Path, value: &Value, findings: &mut Vec<LintFinding>) {
    let Some(obj) = value.as_object() else {
        push_memory(
            findings,
            &format!("{}: top-level must be object", path.display()),
        );
        return;
    };
    let table_shape = obj.contains_key("tables")
        || (obj.contains_key("version")
            && !obj.contains_key("collections")
            && !obj.contains_key("fields"));
    if table_shape {
        check_table_schema(path, obj, findings);
        return;
    }
    if let Some(collections) = obj.get("collections") {
        check_legacy_collections(path, collections, findings);
    }
    if let Some(fields) = obj.get("fields").and_then(Value::as_object) {
        check_legacy_fields(path, "<root>", fields, findings);
    }
}

fn check_table_schema(path: &Path, obj: &Map<String, Value>, findings: &mut Vec<LintFinding>) {
    match obj.get("version").and_then(Value::as_str) {
        Some(version) if semver::Version::parse(version).is_ok() => {}
        _ => push_memory(
            findings,
            &format!(
                "{}: memory_schema field \"version\" must be a semver string",
                path.display()
            ),
        ),
    }
    let Some(tables) = obj.get("tables").and_then(Value::as_array) else {
        push_memory(
            findings,
            &format!(
                "{}: memory_schema field \"tables\" must be an array",
                path.display()
            ),
        );
        return;
    };
    for (idx, table) in tables.iter().enumerate() {
        let Some(table) = table.as_object() else {
            push_memory(
                findings,
                &format!(
                    "{}: table \"#{idx}\" field \"table\" must be an object",
                    path.display()
                ),
            );
            continue;
        };
        check_table(path, idx, table, findings);
    }
}

fn check_table(
    path: &Path,
    idx: usize,
    table: &Map<String, Value>,
    findings: &mut Vec<LintFinding>,
) {
    let table_name = match table.get("name").and_then(Value::as_str) {
        Some(name) if !name.trim().is_empty() => name.to_string(),
        _ => {
            let fallback = format!("#{idx}");
            push_table(
                path,
                &fallback,
                "name",
                "must be a non-empty string",
                findings,
            );
            fallback
        }
    };
    let Some(columns) = table.get("columns").and_then(Value::as_array) else {
        push_table(path, &table_name, "columns", "must be an array", findings);
        return;
    };
    let mut column_names = HashSet::new();
    for (col_idx, column) in columns.iter().enumerate() {
        let Some(column) = column.as_object() else {
            push_table(
                path,
                &table_name,
                "columns",
                &format!("column #{col_idx} must be an object"),
                findings,
            );
            continue;
        };
        check_column(path, &table_name, column, &mut column_names, findings);
    }
    check_primary_key(
        path,
        &table_name,
        table.get("primary_key"),
        &column_names,
        findings,
    );
}

fn check_column(
    path: &Path,
    table_name: &str,
    column: &Map<String, Value>,
    column_names: &mut HashSet<String>,
    findings: &mut Vec<LintFinding>,
) {
    let name = match column.get("name").and_then(Value::as_str) {
        Some(name) if !name.trim().is_empty() => name.to_string(),
        _ => {
            push_column(
                path,
                table_name,
                "<unnamed>",
                "name",
                "must be a non-empty string",
                findings,
            );
            return;
        }
    };
    if !column_names.insert(name.clone()) {
        push_column(
            path,
            table_name,
            &name,
            "name",
            "duplicates an earlier column",
            findings,
        );
    }
    match column.get("type").and_then(Value::as_str) {
        Some(ty) if SUPPORTED_TABLE_COLUMN_TYPES.contains(&ty) => {}
        _ => push_column(
            path,
            table_name,
            &name,
            "type",
            "must be one of string, int, float, bool, json, datetime",
            findings,
        ),
    }
    if column
        .get("nullable")
        .is_some_and(|nullable| !nullable.is_boolean())
    {
        push_column(
            path,
            table_name,
            &name,
            "nullable",
            "must be a bool",
            findings,
        );
    }
}

fn check_primary_key(
    path: &Path,
    table_name: &str,
    primary_key: Option<&Value>,
    column_names: &HashSet<String>,
    findings: &mut Vec<LintFinding>,
) {
    let Some(primary_key) = primary_key else {
        return;
    };
    let Some(keys) = primary_key.as_array() else {
        push_table(
            path,
            table_name,
            "primary_key",
            "must be an array of column names",
            findings,
        );
        return;
    };
    for key in keys {
        let Some(key) = key.as_str() else {
            push_table(
                path,
                table_name,
                "primary_key",
                "values must be strings",
                findings,
            );
            continue;
        };
        if !column_names.contains(key) {
            push_table(
                path,
                table_name,
                "primary_key",
                &format!("references unknown column \"{key}\""),
                findings,
            );
        }
    }
}

fn check_legacy_collections(path: &Path, collections: &Value, findings: &mut Vec<LintFinding>) {
    match collections {
        Value::Array(collections) => {
            for collection in collections {
                let name = collection
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("<unnamed>");
                if let Some(fields) = collection.get("fields").and_then(Value::as_object) {
                    check_legacy_fields(path, name, fields, findings);
                }
            }
        }
        Value::Object(collections) => {
            for (name, collection) in collections {
                if let Some(fields) = collection.get("fields").and_then(Value::as_object) {
                    check_legacy_fields(path, name, fields, findings);
                }
            }
        }
        _ => push_memory(
            findings,
            &format!("{}: collections must be an array or object", path.display()),
        ),
    }
}

fn check_legacy_fields(
    path: &Path,
    collection_name: &str,
    fields: &Map<String, Value>,
    findings: &mut Vec<LintFinding>,
) {
    for (field, def) in fields {
        let ty = def
            .as_str()
            .or_else(|| def.get("type").and_then(Value::as_str))
            .unwrap_or("");
        if ty.trim().is_empty() {
            push_memory(
                findings,
                &format!(
                    "{}: collection \"{collection_name}\" field \"{field}\" missing type",
                    path.display()
                ),
            );
        }
    }
}

fn push_table(
    path: &Path,
    table_name: &str,
    field: &str,
    reason: &str,
    findings: &mut Vec<LintFinding>,
) {
    push_memory(
        findings,
        &format!(
            "{}: table \"{table_name}\" field \"{field}\" {reason}",
            path.display()
        ),
    );
}

fn push_column(
    path: &Path,
    table_name: &str,
    column_name: &str,
    field: &str,
    reason: &str,
    findings: &mut Vec<LintFinding>,
) {
    push_memory(
        findings,
        &format!(
            "{}: table \"{table_name}\" column \"{column_name}\" field \"{field}\" {reason}",
            path.display()
        ),
    );
}

fn push_memory(findings: &mut Vec<LintFinding>, message: &str) {
    push(
        findings,
        CheckId::MemorySchema,
        PackLintSeverity::Error,
        message,
    );
}
