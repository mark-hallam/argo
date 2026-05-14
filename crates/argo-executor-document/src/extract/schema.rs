//! JSON-schema subset used by document extraction.
//!
//! The extractor accepts operator-provided JSON Schema, but the first
//! production slice only needs the deterministic subset common to
//! invoices, receipts, and contracts: root object, `properties`,
//! `required`, primitive JSON types, and arrays.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

/// Error returned when an extraction schema cannot be parsed.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum SchemaError {
    /// The schema root must be a JSON object.
    #[error("schema root must be a JSON object")]
    RootNotObject,
    /// The root `type` must be `object` when present.
    #[error("schema root type must be object")]
    RootTypeNotObject,
    /// The schema must define an object-valued `properties` map.
    #[error("schema properties must be a JSON object")]
    PropertiesNotObject,
    /// A property definition was missing a valid JSON `type`.
    #[error("property '{field}' has unsupported or missing type")]
    UnsupportedType { field: String },
}

/// Supported JSON Schema type tags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JsonType {
    /// JSON string.
    String,
    /// JSON number.
    Number,
    /// JSON integer.
    Integer,
    /// JSON boolean.
    Boolean,
    /// JSON array.
    Array,
    /// JSON object.
    Object,
}

impl JsonType {
    fn parse(value: &Value) -> Option<Self> {
        match value.as_str()? {
            "string" => Some(Self::String),
            "number" => Some(Self::Number),
            "integer" => Some(Self::Integer),
            "boolean" => Some(Self::Boolean),
            "array" => Some(Self::Array),
            "object" => Some(Self::Object),
            _ => None,
        }
    }

    /// Return true when `value` conforms to this JSON type.
    pub fn matches(self, value: &Value) -> bool {
        match self {
            Self::String => value.is_string(),
            Self::Number => value.is_number(),
            Self::Integer => value.as_i64().is_some() || value.as_u64().is_some(),
            Self::Boolean => value.is_boolean(),
            Self::Array => value.is_array(),
            Self::Object => value.is_object(),
        }
    }
}

/// One property in the operator-provided extraction schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldSpec {
    /// JSON type required for the extracted field value.
    pub json_type: JsonType,
    /// Whether the field must appear in the provider response.
    pub required: bool,
}

/// Parsed root object schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtractionSchema {
    /// Field specs keyed by output field name.
    pub fields: BTreeMap<String, FieldSpec>,
}

impl ExtractionSchema {
    /// Parse the supported JSON Schema subset.
    pub fn from_json(value: &Value) -> Result<Self, SchemaError> {
        let root = value.as_object().ok_or(SchemaError::RootNotObject)?;
        if let Some(root_type) = root.get("type") {
            if root_type.as_str() != Some("object") {
                return Err(SchemaError::RootTypeNotObject);
            }
        }
        let props = root
            .get("properties")
            .and_then(Value::as_object)
            .ok_or(SchemaError::PropertiesNotObject)?;
        let required = root
            .get("required")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect::<BTreeSet<_>>();
        let mut fields = BTreeMap::new();
        for (name, spec) in props {
            let Some(json_type) = spec.get("type").and_then(JsonType::parse) else {
                return Err(SchemaError::UnsupportedType {
                    field: name.clone(),
                });
            };
            fields.insert(
                name.clone(),
                FieldSpec {
                    json_type,
                    required: required.contains(name.as_str()),
                },
            );
        }
        Ok(Self { fields })
    }

    /// Validate extracted values against required fields and type tags.
    pub fn validate_values(&self, values: &BTreeMap<String, Value>) -> Vec<SchemaViolation> {
        let mut violations = Vec::new();
        for (name, spec) in &self.fields {
            match values.get(name) {
                Some(value) if !spec.json_type.matches(value) => {
                    violations.push(SchemaViolation::WrongType {
                        field: name.clone(),
                        expected: spec.json_type,
                    });
                }
                None if spec.required => {
                    violations.push(SchemaViolation::MissingRequired {
                        field: name.clone(),
                    });
                }
                _ => {}
            }
        }
        violations
    }

    /// Human-readable schema hint for provider prompts.
    pub fn prompt_fields(&self) -> String {
        self.fields
            .iter()
            .map(|(name, spec)| {
                let required = if spec.required {
                    "required"
                } else {
                    "optional"
                };
                format!("{name}: {:?} ({required})", spec.json_type)
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// Validation failure for one extracted field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SchemaViolation {
    /// A required field was absent.
    MissingRequired {
        /// Missing field name.
        field: String,
    },
    /// A field value had the wrong JSON type.
    WrongType {
        /// Field name.
        field: String,
        /// Expected JSON type.
        expected: JsonType,
    },
}

impl SchemaViolation {
    /// Stable text passed back to the provider for the single retry.
    pub fn correction_hint(&self) -> String {
        match self {
            Self::MissingRequired { field } => format!("missing required field '{field}'"),
            Self::WrongType { field, expected } => {
                format!("field '{field}' must be {expected:?}")
            }
        }
    }
}
