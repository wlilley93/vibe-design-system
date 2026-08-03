//! `vds schema`: emit the artefact JSON Schemas, or check the committed ones
//! for drift.
//!
//! The schemas are DERIVED from the Rust types, not maintained beside them. That
//! is the derive-don't-store rule applied to VDS's own contract, and it closes a
//! defect the previous arrangement had: a hand-written schema and a hand-written
//! parser are two opinions about one shape, and two opinions drift. The audit
//! found exactly that, twice: two of the then-six committed schemas described fields
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
    BurndownReading, BurndownRecord, ComponentRecord, DirectionRecord, EXIT_VIOLATION,
    GeometryAuthority, GeometryBound, GeometryReading, LockEntry, Pin, ProhibitionRecord,
    ProofResult, RedrawRecord, Result, RouteBindingLedger, RouteManifest, ScreenRecord, SignOff,
    StageIntent, StagePlan, StageRecord, Submission, VdsError, VisualReviewRecord, Warrant,
    write_text_atomically,
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
    let mut out = BTreeMap::new();
    // A FRESH generator per schema. Sharing one made every schema a function of
    // the emission ORDER rather than of its own type: schemars accumulates
    // `definitions` on the generator, so each artefact absorbed the definitions
    // of every artefact emitted before it. `pin.schema.json` published
    // `Accessibility`, `ArrangementContract` and `CodeCounterpart`, none of which
    // a pin can contain, and reordering two lines in this function would have
    // rewritten five committed files with no type having changed.
    //
    // It was found by adding a tenth artefact: emitting `geometry-bound` second
    // leaked `BoundEntry` and `SurfaceKind` into the five schemas emitted after
    // it, and `vds schema check` reported drift in five files a commit had not
    // touched. A published schema that declares types its artefact cannot hold
    // is a contract that describes more than it means.
    macro_rules! emit {
        ($name:literal, $type:ty) => {
            let root = generator().root_schema_for::<$type>();
            let text = serde_json::to_string_pretty(&root).map_err(|e| VdsError::Serialize {
                what: $name.to_owned(),
                message: e.to_string(),
            })?;
            out.insert($name, text + "\n");
        };
    }
    emit!("component-record", ComponentRecord);
    emit!("screen-record", ScreenRecord);
    emit!("geometry-bound", GeometryBound);
    emit!("geometry-reading", GeometryReading);
    emit!("geometry-authority", GeometryAuthority);
    emit!("prohibition-record", ProhibitionRecord);
    emit!("burndown-record", BurndownRecord);
    emit!("burndown-reading", BurndownReading);
    emit!("signoff", SignOff);
    emit!("direction-record", DirectionRecord);
    emit!("redraw-record", RedrawRecord);
    emit!("visual-review-record", VisualReviewRecord);
    emit!("route-manifest", RouteManifest);
    emit!("stage-record", StageRecord);
    emit!("route-binding-ledger", RouteBindingLedger);
    // THE TWO ARTEFACTS THAT LIVE IN THE SUBSCRIBER TREE, published here for
    // exactly that reason. They carry boxes and paints, so they may not live
    // under `.vds/**` (VDS S-2(2); [2026] VJS-FI-VDS 1 orders 2 and 4), and a
    // shape a subscriber has to author outside the record is a shape VDS has to
    // publish or nobody can write one. Their schemas are the only two here that
    // describe a file this engine refuses to hold.
    emit!("stage-intent", StageIntent);
    emit!("stage-plan", StagePlan);
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
                println!("  DRIFTED  {name}.schema.json differs from what the type generates");
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

    /// Every emitted schema is committed, and every committed schema is emitted.
    ///
    /// This used to assert a HARD-CODED COUNT of seven, and the count rotted the moment the
    /// twelfth proof kind landed with two artefacts of its own: nine schemas emitted, the
    /// test still demanding seven, and the failure message still explaining why seven was
    /// the right number. That is the count-restated-instead-of-derived failure this
    /// repository has now had in four places, and the fix is the same every time - ask the
    /// artefacts, not a number somebody typed.
    ///
    /// Pairing both directions is what makes it non-circular. Comparing the generator with
    /// itself would pass on any count at all; comparing it with the COMMITTED directory
    /// catches a schema emitted and never committed, and one committed after its type was
    /// deleted.
    #[test]
    fn every_emitted_schema_is_committed_and_each_is_valid_json() {
        let generated = schemas().unwrap();
        assert!(
            !generated.is_empty(),
            "the generator emitted nothing, so every assertion below would pass vacuously"
        );

        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schema");
        let committed: std::collections::BTreeSet<String> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                e.file_name()
                    .to_str()
                    .and_then(|n| n.strip_suffix(".schema.json"))
                    .map(str::to_owned)
            })
            .collect();
        let emitted: std::collections::BTreeSet<String> =
            generated.keys().map(|k| (*k).to_owned()).collect();

        let uncommitted: Vec<&String> = emitted.difference(&committed).collect();
        assert!(
            uncommitted.is_empty(),
            "these schemas are generated and not committed, so nothing publishes them: \
             {uncommitted:?}. Run `vds schema emit`."
        );
        let orphaned: Vec<&String> = committed.difference(&emitted).collect();
        assert!(
            orphaned.is_empty(),
            "these schemas are committed and no longer generated, so they publish a contract \
             for an artefact this build cannot produce: {orphaned:?}"
        );

        for (name, text) in &generated {
            let value: serde_json::Value = serde_json::from_str(text)
                .unwrap_or_else(|e| panic!("{name} is not valid JSON: {e}"));
            assert!(
                value.get("properties").is_some(),
                "{name} has no properties"
            );
        }
    }

    #[test]
    fn generation_is_deterministic() {
        assert_eq!(schemas().unwrap(), schemas().unwrap());
    }

    /// A closed vocabulary and a new field reach the PUBLISHED contract, or the
    /// JSON and the Rust are two opinions about one shape.
    ///
    /// Named values rather than a count, for the reason the test above records:
    /// a count rots the moment a type gains a member and the failure message
    /// still explains why the old number was right. These two are the ones a
    /// consumer validates against - a delta the engine will refuse to
    /// deserialise, and the band declaration without which the correspondence
    /// rule cannot run - so a subscriber that reads only the schema must be
    /// able to see both.
    #[test]
    fn the_published_schemas_carry_the_delta_vocabulary_and_the_band_declaration() {
        let generated = schemas().unwrap();
        let review = &generated["visual-review-record"];
        for disposition in vds_core::DeltaDisposition::ALL {
            assert!(
                review.contains(&format!("\"{}\"", disposition.as_str())),
                "the visual review schema does not publish the {disposition} disposition, so a \
                 consumer validating against it would accept a record this engine refuses"
            );
        }
        assert!(
            review.contains("forbidden_by_policy"),
            "the closed disposition vocabulary is not in the published contract"
        );
        // Read as VALUES and not as text. Both words appear, correctly, in the
        // prose explaining why neither is a member, and a substring test would
        // have failed on the sentence that says they do not exist.
        let parsed: serde_json::Value = serde_json::from_str(review).unwrap();
        // One `oneOf` branch per variant, because every variant carries its own
        // documentation: the member is the branch's single-valued `enum`.
        let members: Vec<String> = parsed["definitions"]["DeltaDisposition"]["oneOf"]
            .as_array()
            .expect("the disposition vocabulary is published as a closed set")
            .iter()
            .filter_map(|branch| branch["enum"][0].as_str().map(str::to_owned))
            .collect();
        assert_eq!(
            members.len(),
            vds_core::DeltaDisposition::ALL.len(),
            "the published vocabulary and the Rust one are different sizes: {members:?}"
        );
        for absent in ["accepted", "wont_fix"] {
            assert!(
                !members.iter().any(|m| m == absent),
                "{absent:?} has reached the published contract, and an acceptance state is \
                 taste exercised downstream of sign-off"
            );
        }
        assert!(
            generated["screen-record"].contains("\"bands\""),
            "the screen record schema does not publish `bands`, and a screen that cannot \
             declare its bands makes the band correspondence permanently unrunnable"
        );
    }

    /// THE ELEVENTH ARTEFACT KIND, AND THE SIXTEENTH PROOF KIND, REACH THE
    /// PUBLISHED CONTRACT.
    ///
    /// Named values rather than a count, for the reason the count test above
    /// records. The three that matter are the ones a consumer validates
    /// against: the closed operation vocabulary, whose closure is the strongest
    /// thing this capability has; the three-valued gate reading, whose third
    /// value is what stops "could not run" reading as a pass; and the intent
    /// path, whose placement outside `.vds/` is a rule of law.
    #[test]
    fn the_published_schemas_carry_the_staged_write_record_and_its_closed_vocabularies() {
        let generated = schemas().unwrap();
        let record = &generated["stage-record"];
        for property in ["\"route\"", "\"target\"", "\"intentPath\"", "\"gates\""] {
            assert!(
                record.contains(property),
                "the stage record schema does not publish {property}, so a consumer validating \
                 against it would accept a record this engine refuses"
            );
        }

        let parsed: serde_json::Value = serde_json::from_str(record).unwrap();
        let readings: Vec<String> = parsed["definitions"]["GateReading"]["oneOf"]
            .as_array()
            .expect("the gate reading vocabulary is published as a closed set")
            .iter()
            .filter_map(|branch| branch["enum"][0].as_str().map(str::to_owned))
            .collect();
        assert_eq!(
            readings.len(),
            vds_core::GateReading::ALL.len(),
            "the published gate vocabulary and the Rust one are different sizes: {readings:?}"
        );
        assert!(
            readings.iter().any(|r| r == "could_not_run"),
            "the third value is what stops a rule with no basis to run reading as a pass, and a \
             consumer that cannot see it will treat one as the other: {readings:?}"
        );
        let gates: Vec<String> = parsed["definitions"]["StageGate"]["oneOf"]
            .as_array()
            .expect("the gate vocabulary is published as a closed set")
            .iter()
            .filter_map(|branch| branch["enum"][0].as_str().map(str::to_owned))
            .collect();
        assert_eq!(gates.len(), vds_core::StageGate::ALL.len(), "{gates:?}");

        // THE CLOSURE, in the published contract. A page-level or frame-level
        // delete reaching the schema is the 2026-07-25 loss becoming
        // repeatable through the sanctioned path, and a consumer generating
        // from this file would then generate one.
        let plan: serde_json::Value = serde_json::from_str(&generated["stage-plan"]).unwrap();
        let operations: Vec<String> = plan["definitions"]["StageOperation"]["oneOf"]
            .as_array()
            .expect("the operation vocabulary is published as a closed set")
            .iter()
            .filter_map(|branch| {
                branch["properties"]["op"]["enum"][0]
                    .as_str()
                    .map(str::to_owned)
            })
            .collect();
        assert_eq!(
            operations.len(),
            6,
            "the operation vocabulary is CLOSED at six: {operations:?}"
        );
        assert!(operations.iter().any(|o| o == "delete_band"));
        for absent in ["delete_page", "delete_frame", "delete_node", "replace_page"] {
            assert!(
                !operations.iter().any(|o| o == absent),
                "{absent:?} has reached the published contract. The 2026-07-25 loss came from a \
                 delete-page-and-recreate step, and a verb in the schema is a verb somebody will \
                 generate a client for."
            );
        }
    }

    /// The two schemas here that describe a file `.vds/` may NOT hold.
    ///
    /// The stage intent and the stage plan carry boxes and resolved paints, so
    /// they live in the subscriber tree (VDS S-2(2); [2026] VJS-FI-VDS 1 orders
    /// 2 and 4). Their shapes are published anyway, because a subscriber has to
    /// author one and a shape nobody publishes is a shape everybody guesses.
    const SUBSCRIBER_TREE_SCHEMAS: [&str; 2] = ["stage-intent", "stage-plan"];

    /// The split, held at the schema level rather than in a paragraph.
    #[test]
    fn the_stage_record_publishes_no_field_the_intent_publishes() {
        let generated = schemas().unwrap();
        let record = &generated["stage-record"];
        for property in ["\"boxOf\"", "\"panes\"", "\"paint\"", "\"resolved\""] {
            assert!(
                !record.contains(property),
                "the stage record schema publishes {property}. The record and the intent split \
                 on LAW rather than on tidiness: a box or a paint under `.vds/**` fails \
                 `no_stored_values` R1 and R3 forever, on a file VDS wrote itself, and a record \
                 is never deleted."
            );
        }
        // And the intent does publish them, or the split has been made by
        // deleting the capability rather than by placing it.
        let intent = &generated["stage-intent"];
        assert!(
            intent.contains("\"boxOf\""),
            "the intent must carry the boxes"
        );
        assert!(
            intent.contains("\"paint\""),
            "the intent must carry the paints"
        );
    }

    /// VDS S-2(4): an artefact may hold a requirement and never a realisation.
    /// The published contract must not name one either.
    #[test]
    fn no_generated_schema_declares_a_property_named_like_a_realisation() {
        let forbidden = [
            "colour",
            "color",
            "hex",
            "rgb",
            "hsl",
            "oklch",
            "fontFamily",
            "fontSize",
            "lineHeight",
            "letterSpacing",
            "radius",
            "borderRadius",
            "shadow",
            "boxShadow",
            "duration",
            "easing",
            "cubicBezier",
            "spacing",
            "px",
            "rem",
            "opacity",
        ];
        for (name, text) in schemas().unwrap() {
            if SUBSCRIBER_TREE_SCHEMAS.contains(&name) {
                // These two describe files VDS refuses to hold, precisely
                // BECAUSE they carry realisation. Holding them to the rule that
                // exists to keep realisation out of `.vds/` would be applying
                // it to the place it sent them. The rule that binds them is the
                // one above: nothing they carry may appear on the record.
                continue;
            }
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
