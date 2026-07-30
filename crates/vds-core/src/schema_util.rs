//! Helpers for hand-written `JsonSchema` impls.
//!
//! The JSON Schemas under `schema/` are GENERATED from these types rather
//! than maintained beside them. That is the derive-don't-store discipline
//! applied to VDS's own contract: a schema stored next to the type it describes
//! is a second opinion that drifts, and VDS exists because a second opinion that
//! drifts is exactly what produced its founding defects.
//!
//! A newtype that carries a lexical constraint (a digest, an identifier, a
//! timestamp) declares that constraint here, once, so the Rust parser and the
//! published schema cannot disagree about it.

use schemars::schema::{InstanceType, Metadata, Schema, SchemaObject, StringValidation};

/// A `{"type": "string", "pattern": ..., "description": ...}` schema.
pub fn pattern_string(pattern: &str, description: &str) -> Schema {
    SchemaObject {
        instance_type: Some(InstanceType::String.into()),
        string: Some(Box::new(StringValidation {
            pattern: Some(pattern.to_owned()),
            ..Default::default()
        })),
        metadata: Some(Box::new(Metadata {
            description: Some(description.to_owned()),
            ..Default::default()
        })),
        ..Default::default()
    }
    .into()
}

/// A `{"type": "string", "format": "date-time", ...}` schema.
pub fn date_time_string(description: &str) -> Schema {
    SchemaObject {
        instance_type: Some(InstanceType::String.into()),
        format: Some("date-time".to_owned()),
        string: Some(Box::new(StringValidation {
            pattern: Some(r"^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$".to_owned()),
            ..Default::default()
        })),
        metadata: Some(Box::new(Metadata {
            description: Some(description.to_owned()),
            ..Default::default()
        })),
        ..Default::default()
    }
    .into()
}
