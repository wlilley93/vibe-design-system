//! `vds schema`: emit the artefact JSON Schemas, or check the committed ones
//! for drift.
//!
//! The schemas are DERIVED from the Rust types, not maintained beside them. That
//! is the derive-don't-store rule applied to VDS's own contract, and it closes a
//! defect the previous arrangement had: a hand-written schema and a hand-written
//! parser are two opinions about one shape, and two opinions drift. The audit
//! found exactly that, twice: two of the six committed schemas described fields
//! no code read, and one described a Figma node id in a form the tool's own help
//! text told authors to produce and the schema then rejected.
//!
//! `vds schema check` is the gate. It regenerates and diffs, so a type change
//! that is not reflected in the committed schema is a failing check rather than
//! a discovery six months later.

use std::collections::BTreeMap;
use std::path::PathBuf;

use clap::{Args as ClapArgs, Subcommand};
use schemars::r#gen::{SchemaGenerator, SchemaSettings};
use vds_core::{
    ComponentRecord, EXIT_VIOLATION, LockEntry, Pin, ProofResult, Result, Submission, VdsError,
    Warrant, write_text_atomically,
};

use crate::{Context, PASSED};

#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand)]
enum Action {
    /// Write `schema/*.schema.json` from the Rust types.
    Emit {
        /// Where to write. Defaults to `schema/` beside the project root.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Regenerate and diff against what is committed. Non-zero on drift.
    Check {
        #[arg(long)]
        dir: Option<PathBuf>,
    },
}

/// The one generator configuration, so `emit` and `check` cannot disagree about
/// what "the schema" is.
fn generator() -> SchemaGenerator {
    let mut settings = SchemaSettings::draft2019_09();
    // An `Option<T>` field means "may be null", not "may be absent". Every
    // artefact field that can be null is null in the record rather than missing
    // from it, so a reader never has to distinguish the two.
    settings.option_add_null_type = true;
    settings.option_nullable = false;
    settings.into_generator()
}

fn schemas() -> Result<BTreeMap<&'static str, String>> {
    let mut generator = generator();
    let mut out = BTreeMap::new();
    macro_rules! emit {
        ($name:literal, $type:ty) => {
            let root = generator.root_schema_for::<$type>();
            let text = serde_json::to_string_pretty(&root).map_err(|e| VdsError::Serialize {
                what: $name.to_owned(),
                message: e.to_string(),
            })?;
            out.insert($name, text + "\n");
        };
    }
    emit!("component-record", ComponentRecord);
    emit!("warrant", Warrant);
    emit!("proof-result", ProofResult);
    emit!("pin", Pin);
    emit!("submission", Submission);
    emit!("enforcement-lock-entry", LockEntry);
    Ok(out)
}

pub fn run(ctx: &Context, args: &Args) -> Result<i32> {
    match &args.action {
        Action::Emit { out } => {
            let dir = resolve_dir(ctx, out.clone())?;
            let generated = schemas()?;
            for (name, text) in &generated {
                let path = dir.join(format!("{name}.schema.json"));
                write_text_atomically(&path, text)?;
                println!("wrote {}", path.display());
            }
            println!();
            println!(
                "{} schemas, generated from the Rust types. Do not hand-edit them: \
                 `vds schema check` regenerates and diffs, so a hand edit is a failing check.",
                generated.len()
            );
            Ok(PASSED)
        }
        Action::Check { dir } => {
            let dir = resolve_dir(ctx, dir.clone())?;
            let generated = schemas()?;
            let mut drifted = Vec::new();
            let mut missing = Vec::new();

            for (name, text) in &generated {
                let path = dir.join(format!("{name}.schema.json"));
                match std::fs::read_to_string(&path) {
                    Err(_) => missing.push(name.to_string()),
                    Ok(committed) if committed.trim() != text.trim() => {
                        drifted.push(name.to_string())
                    }
                    Ok(_) => {}
                }
            }

            // A committed schema with no type behind it is the other direction
            // of the same drift: a contract nothing produces and nothing reads.
            let mut orphaned = Vec::new();
            if dir.is_dir() {
                for entry in std::fs::read_dir(&dir).map_err(|e| VdsError::io(dir.display(), e))? {
                    let entry = entry.map_err(|e| VdsError::io(dir.display(), e))?;
                    let name = entry.file_name().to_string_lossy().into_owned();
                    if let Some(stem) = name.strip_suffix(".schema.json")
                        && !generated.contains_key(stem)
                    {
                        orphaned.push(name);
                    }
                }
            }
            orphaned.sort();

            if drifted.is_empty() && missing.is_empty() && orphaned.is_empty() {
                println!(
                    "{} schemas in {} match the types they are generated from.",
                    generated.len(),
                    dir.display()
                );
                return Ok(PASSED);
            }

            println!("SCHEMA DRIFT in {}:", dir.display());
            for name in &missing {
                println!("  MISSING  {name}.schema.json is generated by this build and is absent");
            }
            for name in &drifted {
                println!(
                    "  DRIFTED  {name}.schema.json differs from what the type generates"
                );
            }
            for name in &orphaned {
                println!(
                    "  ORPHAN   {name} has no type behind it: a contract nothing produces and \
                     nothing reads"
                );
            }
            println!();
            println!("Regenerate with: vds schema emit");
            Ok(EXIT_VIOLATION)
        }
    }
}

fn resolve_dir(ctx: &Context, explicit: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(dir) = explicit {
        return Ok(dir);
    }
    // Prefer the project root when there is one, so `vds schema emit` inside a
    // subscriber writes into that project rather than wherever the binary lives.
    match ctx.project() {
        Ok(project) => Ok(project.root.join("schema")),
        Err(_) => Ok(PathBuf::from("schema")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn six_schemas_are_generated_and_each_is_valid_json() {
        let generated = schemas().unwrap();
        assert_eq!(generated.len(), 6);
        for (name, text) in &generated {
            let value: serde_json::Value = serde_json::from_str(text)
                .unwrap_or_else(|e| panic!("{name} is not valid JSON: {e}"));
            assert!(value.get("properties").is_some(), "{name} has no properties");
        }
    }

    #[test]
    fn generation_is_deterministic() {
        assert_eq!(schemas().unwrap(), schemas().unwrap());
    }

    /// VDS S-2(4): an artefact may hold a requirement and never a realisation.
    /// The published contract must not name one either.
    #[test]
    fn no_generated_schema_declares_a_property_named_like_a_realisation() {
        let forbidden = [
            "colour", "color", "hex", "rgb", "hsl", "oklch", "fontFamily", "fontSize",
            "lineHeight", "letterSpacing", "radius", "borderRadius", "shadow", "boxShadow",
            "duration", "easing", "cubicBezier", "spacing", "px", "rem", "opacity",
        ];
        for (name, text) in schemas().unwrap() {
            for property in forbidden {
                assert!(
                    !text.contains(&format!("\"{property}\": {{")),
                    "{name} declares a property named {property:?}"
                );
            }
        }
    }

    #[test]
    fn the_component_record_schema_pins_the_identifier_pattern() {
        let generated = schemas().unwrap();
        let text = &generated["component-record"];
        assert!(text.contains("^CMP-[0-9]{4}$"), "{text}");
    }

    #[test]
    fn the_proof_result_schema_fixes_capture_mode_to_one_value() {
        let generated = schemas().unwrap();
        let text = &generated["proof-result"];
        assert!(text.contains("automatic"), "{text}");
    }
}
