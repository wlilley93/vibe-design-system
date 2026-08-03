//! The staged write's diff engine and plan emitter (draft S-7E).
//!
//! # What this is, and the one thing it is not
//!
//! It reads a SAVED capture of one frame, compares it against a
//! [`StageIntent`], and emits the [`StageOperation`]s that would bring the frame
//! to the intent. It is not, and must never be described as, a control that
//! stops anyone writing to Figma directly: the REST API cannot write document
//! nodes at all, so VDS holds no privileged channel it could withhold, and its
//! own apply goes through the same plugin bridge every agent already has. What
//! is new here is that the operation list EXISTS, on disk, before anything
//! reaches the canvas.
//!
//! Out of band and from a saved response, exactly as [`crate::frames`] is, and
//! for the same reason: VDS S-7(2)(1) forbids a network call inside a proof, and
//! the gates that read a plan are a proof.
//!
//! # THE DIFF IS KEYED ON BAND NAME, NOT ON NODE ID, AND THAT IS WHAT MAKES IT
//! IDEMPOTENT
//!
//! A node id changes when a node is recreated. Keyed on node id, the second run
//! over the same intent finds every band missing and recreates all of them, so
//! an "idempotent" apply would in fact rebuild the frame every time and destroy
//! anything a designer had put inside a band. Keyed on the CLOSED-vocabulary
//! band name, the second run finds what the first created and emits ZERO
//! operations.
//!
//! That is why the naming gate is load-bearing rather than cosmetic: the closed
//! vocabulary is not tidiness, it is the identity function of the diff. And it
//! is why [`StageOperation::SetName`] moves a band's spelling onto the canonical
//! one rather than to an arbitrary string - the identity is the region PARSED
//! from the name, so a rename that could change the parse would change the key
//! mid-apply.
//!
//! # PER BAND, ONLY DECLARED FIELDS ARE COMPARED
//!
//! A field the intent does not declare is neither compared nor written, so a
//! designer's hand-added note inside a band survives an apply. A child of the
//! frame whose name is NOT in the closed vocabulary is not a band at all: it is
//! recorded in [`FrameReading::untouched`] and no operation can reach it.

use std::collections::{BTreeMap, BTreeSet};

use vds_core::{
    BandBox, Digest, PlanChunk, Result, ReviewRegion, StageId, StageIntent, StageOperation,
    StagePlan, Timestamp, VdsError,
};
use vds_css::colour::Colour;

pub const EMITTER_COMMAND: &str = "vds stage plan";

/// The character budget one chunk's operations may occupy.
///
/// THERE IS NO ATOMICITY AND THIS CONSTANT IS WHY. The plugin bridge caps a
/// single call's `code` argument at fifty thousand characters and offers no
/// transaction, so a large frame's plan cannot go over in one call and must be
/// split. Splitting reintroduces a partial-apply window: the third chunk can
/// fail after the first two landed, and nothing rolls them back.
///
/// The budget here is a fraction of the cap rather than the cap itself, because
/// what crosses the bridge is a PROGRAM that carries the operations, not the
/// operations' own serialised bytes, and a program is several times longer than
/// its data. A budget set at the cap would produce chunks that are refused at
/// the far end, which is the same partial apply with a worse error message.
pub const CHUNK_CHARACTER_BUDGET: usize = 12_000;

/// The bridge's own limit, recorded so the fraction above is checkable rather
/// than a number somebody liked.
pub const BRIDGE_CODE_LIMIT: usize = 50_000;

/// One band as the CAPTURE currently draws it.
#[derive(Debug, Clone, PartialEq)]
pub struct BandState {
    /// The identity key: the region parsed from the layer's name.
    pub band: ReviewRegion,
    /// The layer's name as the file spells it, which may be a variant of the
    /// canonical spelling.
    pub layer_name: String,
    /// The Figma node id. CARRIED AND NEVER KEYED ON. It is here so a reader of
    /// a plan can find the layer, and the idempotence test changes every one of
    /// them between runs to prove the diff does not depend on it.
    pub node_id: String,
    /// Frame-relative, so an intent's boxes do not have to know where on the
    /// canvas the frame happens to sit.
    pub box_of: BandBox,
    /// The band's first visible solid fill, or `None` where it has none this
    /// reader can resolve.
    pub paint: Option<Colour>,
    /// Position among the frame's BANDS, counting only children the closed
    /// vocabulary names.
    pub index: u32,
}

/// One frame, as a saved capture draws it.
#[derive(Debug, Clone, PartialEq)]
pub struct FrameReading {
    pub node_id: String,
    pub frame_name: String,
    pub frame_box: BandBox,
    pub bands: Vec<BandState>,
    /// Children whose names the closed vocabulary does NOT name.
    ///
    /// Recorded so a plan can publish them, and unreachable by every operation:
    /// a node VDS did not create and the intent does not name is never touched.
    pub untouched: Vec<String>,
}

impl FrameReading {
    pub fn band(&self, band: ReviewRegion) -> Option<&BandState> {
        self.bands.iter().find(|b| b.band == band)
    }
}

/// The region a layer name states, or `None` where it states none.
///
/// Normalised rather than matched exactly, because a designer types "Body Rows"
/// and "body-rows" and means the band. A name that normalises to nothing in the
/// closed vocabulary is not a band, and the whole point of that answer is that
/// the layer is then out of every operation's reach.
pub fn band_of(layer_name: &str) -> Option<ReviewRegion> {
    let normalised: String = layer_name
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c == ' ' || c == '-' { '_' } else { c })
        .collect();
    ReviewRegion::parse(&normalised)
}

/// Read one frame out of a saved `GET /v1/files/:key/nodes` response.
///
/// `Ok(None)` where the capture does not carry that node. `None` is NOT "the
/// frame is empty": a caller that cannot see a frame has to say so, or a diff
/// taken against nothing emits a create for every band and rebuilds a frame
/// that was already right.
pub fn read_frame(body: &str, node_id: &str) -> Result<Option<FrameReading>> {
    let payload: serde_json::Value = serde_json::from_str(body).map_err(|e| {
        VdsError::precondition(format!(
            "the capture is not JSON: {e}. A partial parse would produce a reading claiming the \
             frame draws fewer bands than it does, and the diff against it would emit a create \
             for each one."
        ))
    })?;
    let nodes = payload
        .get("nodes")
        .and_then(|v| v.as_object())
        .ok_or_else(|| {
            VdsError::precondition(
                "the capture carries no `nodes` object. This reads a saved \
                 `GET /v1/files/:key/nodes` response, which is the endpoint that takes a depth; \
                 a `GET /v1/files/:key` response has a `document` instead and is a different \
                 shape answering a different question.",
            )
        })?;
    let wanted = crate::frames::normalise_node_id(node_id);
    let Some(document) = nodes.iter().find_map(|(id, wrapper)| {
        (crate::frames::normalise_node_id(id) == wanted).then(|| wrapper.get("document"))?
    }) else {
        return Ok(None);
    };
    Ok(Some(reading_of(&wanted, document)))
}

fn reading_of(node_id: &str, document: &serde_json::Value) -> FrameReading {
    let frame_box = box_of(document).unwrap_or(BandBox {
        x: 0.0,
        y: 0.0,
        width: 0.0,
        height: 0.0,
    });
    let mut bands = Vec::new();
    let mut untouched = Vec::new();
    let children = document
        .get("children")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    for child in &children {
        // Figma omits `visible` when it is true. An invisible layer is a
        // drawing the designer switched off, and staging over one would write
        // into something nobody can see in the file.
        if child.get("visible").and_then(|v| v.as_bool()) == Some(false) {
            continue;
        }
        let name = child
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_owned();
        let Some(band) = band_of(&name) else {
            untouched.push(name);
            continue;
        };
        let absolute = box_of(child).unwrap_or(BandBox {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        });
        bands.push(BandState {
            band,
            layer_name: name,
            node_id: child
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_owned(),
            box_of: BandBox {
                x: absolute.x - frame_box.x,
                y: absolute.y - frame_box.y,
                width: absolute.width,
                height: absolute.height,
            },
            paint: paint_of(child),
            // Replaced below, once the band children are known.
            index: 0,
        });
    }
    for (index, band) in bands.iter_mut().enumerate() {
        band.index = index as u32;
    }
    FrameReading {
        node_id: node_id.to_owned(),
        frame_name: document
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_owned(),
        frame_box,
        bands,
        untouched,
    }
}

fn box_of(value: &serde_json::Value) -> Option<BandBox> {
    let b = value.get("absoluteBoundingBox")?;
    Some(BandBox {
        x: b.get("x")?.as_f64()?,
        y: b.get("y")?.as_f64()?,
        width: b.get("width")?.as_f64()?,
        height: b.get("height")?.as_f64()?,
    })
}

/// The node's first visible SOLID fill, or `None`.
///
/// `None` and never a default. A gradient, an image or an absent fill is a
/// paint this reader cannot compare, and answering "black" for it would emit a
/// set-paint on every run and make the apply permanently non-idempotent while
/// looking like a real finding.
fn paint_of(value: &serde_json::Value) -> Option<Colour> {
    let fills = value.get("fills")?.as_array()?;
    for fill in fills {
        if fill.get("visible").and_then(|v| v.as_bool()) == Some(false) {
            continue;
        }
        if fill.get("type").and_then(|v| v.as_str()) != Some("SOLID") {
            continue;
        }
        let c = fill.get("color")?;
        let alpha = fill
            .get("opacity")
            .and_then(|v| v.as_f64())
            .unwrap_or_else(|| c.get("a").and_then(|v| v.as_f64()).unwrap_or(1.0));
        return Colour::new(
            c.get("r")?.as_f64()?,
            c.get("g")?.as_f64()?,
            c.get("b")?.as_f64()?,
            alpha,
        )
        .ok();
    }
    None
}

// ------------------------------------------------------------------ the diff

/// Everything a diff needs, in one place.
pub struct DiffInputs<'a> {
    pub intent: &'a StageIntent,
    pub reading: &'a FrameReading,
    /// Custom property name, WITHOUT its leading dashes, to what the shipped
    /// stylesheet resolves it to in its base scope.
    ///
    /// Supplied rather than resolved here, so this crate never has to decide
    /// which stylesheet is the record and so the same map the contrast gate
    /// measured is the map the diff writes from. A property missing from it is
    /// a property the gate could not resolve, and the diff emits no paint for
    /// it rather than writing a guess.
    pub paints: &'a BTreeMap<String, Colour>,
}

/// How close two lengths must be to count as the same length.
///
/// Not zero. Figma returns absolute bounding boxes as floating point and a
/// round trip through a transform moves the last bit, so an exact comparison
/// emits a set-box on every run and the apply is never idempotent. Half a unit
/// is below anything a person can see and above everything a round trip
/// introduces.
const SAME_LENGTH: f64 = 0.5;

fn same(a: f64, b: f64) -> bool {
    (a - b).abs() <= SAME_LENGTH
}

fn same_box(a: &BandBox, b: &BandBox) -> bool {
    same(a.x, b.x) && same(a.y, b.y) && same(a.width, b.width) && same(a.height, b.height)
}

/// The property name a floor or a paint states, without its leading dashes.
pub fn property_key(raw: &str) -> String {
    raw.trim().trim_start_matches("--").to_owned()
}

/// The operations that would bring the frame to the intent.
///
/// EMPTY where the frame already is the intent. That is the whole contract, and
/// it is MEASURED by a test that runs this twice rather than asserted in this
/// sentence.
///
/// The order is load-bearing: deletes, then creates, then the per-band edits,
/// then reorders. A reorder is computed over the bands the INTENT declares, so
/// it has to follow the two operations that decide which bands those are.
pub fn diff(inputs: &DiffInputs) -> Vec<StageOperation> {
    let intent = inputs.intent;
    let reading = inputs.reading;
    let mut out = Vec::new();

    let declared: BTreeSet<ReviewRegion> = intent.declared_bands().into_iter().collect();

    // DELETE. Two conditions, checked together and never separately: the band's
    // name is in the closed vocabulary (so it is a band at all), and the intent
    // no longer declares it. Anything else in the frame is in `untouched` and
    // no operation here can reach it. There is no page-level and no frame-level
    // delete, and the reason is in the enum's own doc: the 2026-07-25 loss came
    // from a delete-page-and-recreate step, and widening the vocabulary makes
    // that loss repeatable through the sanctioned path.
    for state in &reading.bands {
        if !declared.contains(&state.band) {
            out.push(StageOperation::DeleteBand { band: state.band });
        }
    }

    // CREATE. A band the intent declares with a box and the frame does not
    // have. A declared band with NO box cannot be created: there is nowhere to
    // put it, and inventing a rectangle would be VDS deciding a realisation.
    for band in &intent.bands {
        if reading.band(band.band).is_some() {
            continue;
        }
        if let Some(box_of) = band.box_of {
            out.push(StageOperation::CreateBand {
                band: band.band,
                box_of,
            });
        }
    }

    // PER BAND, ONLY DECLARED FIELDS.
    for band in &intent.bands {
        let existing = reading.band(band.band);

        // The canonical spelling. Idempotent by construction: once the name is
        // the region's own string it parses to the same region and compares
        // equal. This is the limb that keeps the diff key stable, which is why
        // it exists at all.
        if let Some(state) = existing
            && state.layer_name != band.band.as_str()
        {
            out.push(StageOperation::SetName {
                band: band.band,
                to: band.band.as_str().to_owned(),
            });
        }

        if let Some(box_of) = band.box_of
            && let Some(state) = existing
            && !same_box(&state.box_of, &box_of)
        {
            out.push(StageOperation::SetBox {
                band: band.band,
                box_of,
            });
        }

        if let Some(paint) = &band.paint
            && let Some(wanted) = inputs.paints.get(&property_key(&paint.property))
        {
            let differs = match existing.and_then(|s| s.paint.as_ref()) {
                // A band with no readable solid fill is painted, because the
                // intent declares one and nothing on the canvas answers it.
                None => true,
                Some(current) => current.quantise_8bit() != wanted.quantise_8bit(),
            };
            if differs {
                out.push(StageOperation::SetPaint {
                    band: band.band,
                    property: paint.property.clone(),
                    resolved: wanted.to_css_hex(),
                });
            }
        }

        if let Some(order) = band.order
            && let Some(state) = existing
            && state.index != order
        {
            out.push(StageOperation::Reorder {
                band: band.band,
                to: order,
            });
        }
    }

    out
}

// ------------------------------------------------------------------ the plan

/// Emit the plan: the operation list, on disk, BEFORE anything reaches the
/// canvas.
///
/// `reading_digest` is the digest of the capture the diff was taken against, so
/// an apply cannot use a plan computed from a frame reading it never saw.
#[allow(clippy::too_many_arguments)]
pub fn emit_plan(
    stage: &StageId,
    intent: &StageIntent,
    intent_digest: Digest,
    reading: &FrameReading,
    reading_path: &str,
    reading_digest: Digest,
    operations: Vec<StageOperation>,
    at: Timestamp,
) -> Result<StagePlan> {
    let mut chunks = Vec::new();
    let mut current: Vec<StageOperation> = Vec::new();
    let mut budget = 0usize;
    for operation in operations {
        let cost = vds_core::canonical_json(&operation)?.len();
        // At least one operation per chunk, always. An operation larger than
        // the whole budget still has to go somewhere, and dropping it would
        // make the plan quietly narrower than the diff.
        if !current.is_empty() && budget + cost > CHUNK_CHARACTER_BUDGET {
            chunks.push(finish_chunk(
                chunks.len() as u32 + 1,
                std::mem::take(&mut current),
            )?);
            budget = 0;
        }
        budget += cost;
        current.push(operation);
    }
    if !current.is_empty() {
        chunks.push(finish_chunk(chunks.len() as u32 + 1, current)?);
    }

    let mut plan = StagePlan {
        schema_version: vds_core::STAGE_PLAN_SCHEMA_VERSION,
        stage: stage.clone(),
        route: intent.route.clone(),
        file_key: intent.file_key.clone(),
        node_id: crate::frames::normalise_node_id(&intent.node_id),
        emitted_by: EMITTER_COMMAND.to_owned(),
        emitted_at: at,
        reading: reading_path.to_owned(),
        reading_digest,
        intent_digest,
        untouched: reading.untouched.clone(),
        chunks,
        content_digest: Digest::of_text("placeholder"),
    };
    plan.content_digest = plan.compute_content_digest()?;
    Ok(plan)
}

fn finish_chunk(ordinal: u32, operations: Vec<StageOperation>) -> Result<PlanChunk> {
    let digest = PlanChunk::compute_digest(ordinal, &operations)?;
    Ok(PlanChunk {
        ordinal,
        operations,
        digest,
    })
}

// --------------------------------------------------------------- G3's input

/// The canonical shell this estate's screens are drawn in.
///
/// LENGTHS, AND THEY LIVE IN CODE FOR THAT REASON, exactly as the frame
/// generator's clustering thresholds do. `.vds/config.toml` is scanned by
/// `no_stored_values`, which fails on a number carrying a CSS length unit
/// anywhere under the record, so a shell dimension in the config would be the
/// field VDS S-2(2) prohibits. These are derivation parameters and not a
/// decision about how a screen should look: changing one changes what the
/// instrument will admit, not what the design is.
pub const SHELL_WIDTH: f64 = 1400.0;
pub const SHELL_HEIGHT: f64 = 900.0;

/// A synthetic capture document built from an intent's declared boxes, so the
/// column derivation can be run BEFORE the write rather than after it.
///
/// One child is added under each pane, and that is not decoration: the frame
/// generator DERIVES its capture depth from the deepest chain present, so a
/// pane drawn as a leaf would sit exactly on the derived boundary and the
/// reading would come back marked truncated. The frame fixtures in the test
/// harness carry the same child for the same reason. It cannot change the
/// answer, because the clustering reads a container's direct children and the
/// added node is one level below every one of them.
pub fn synthetic_document(intent: &StageIntent) -> serde_json::Value {
    fn node(name: &str, b: &BandBox, children: serde_json::Value) -> serde_json::Value {
        serde_json::json!({
            "id": "0:0",
            "name": name,
            "type": "FRAME",
            "absoluteBoundingBox": {"x": b.x, "y": b.y, "width": b.width, "height": b.height},
            "children": children,
        })
    }
    let mut bands = Vec::new();
    for band in &intent.bands {
        let Some(box_of) = band.box_of else { continue };
        let panes: Vec<serde_json::Value> = band
            .panes
            .iter()
            .map(|pane| {
                // The pane wraps one content child, so the deepest chain is
                // three and the bands sit above the derived boundary.
                let content = node("content", pane, serde_json::json!([]));
                node("pane", pane, serde_json::json!([content]))
            })
            .collect();
        bands.push(node(
            band.band.as_str(),
            &box_of,
            serde_json::Value::Array(panes),
        ));
    }
    node(
        "body",
        &BandBox {
            x: 0.0,
            y: 0.0,
            width: SHELL_WIDTH,
            height: SHELL_HEIGHT,
        },
        serde_json::Value::Array(bands),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use vds_core::{BandIntent, PaintIntent, ReviewRegion, STAGE_INTENT_SCHEMA_VERSION};

    fn box_of(x: f64, y: f64, width: f64, height: f64) -> BandBox {
        BandBox {
            x,
            y,
            width,
            height,
        }
    }

    /// A capture drawing `bands` as `(layer name, node id, box, fill)`.
    fn capture(node_id: &str, bands: &[(&str, &str, BandBox, Option<&str>)]) -> String {
        let children: Vec<serde_json::Value> = bands
            .iter()
            .map(|(name, id, b, fill)| {
                let mut node = serde_json::json!({
                    "id": id,
                    "name": name,
                    "type": "FRAME",
                    "absoluteBoundingBox": {"x": b.x, "y": b.y, "width": b.width, "height": b.height},
                    "children": [],
                });
                if let Some(hex) = fill {
                    let bytes = u32::from_str_radix(hex.trim_start_matches('#'), 16).unwrap();
                    node["fills"] = serde_json::json!([{
                        "type": "SOLID",
                        "color": {
                            "r": f64::from((bytes >> 16) & 0xff) / 255.0,
                            "g": f64::from((bytes >> 8) & 0xff) / 255.0,
                            "b": f64::from(bytes & 0xff) / 255.0,
                        },
                        "opacity": 1.0,
                    }]);
                }
                node
            })
            .collect();
        serde_json::json!({
            "nodes": {
                node_id: {
                    "document": {
                        "id": node_id,
                        "name": "Screen",
                        "type": "FRAME",
                        "absoluteBoundingBox": {"x": 0.0, "y": 0.0, "width": SHELL_WIDTH, "height": SHELL_HEIGHT},
                        "children": children,
                    }
                }
            }
        })
        .to_string()
    }

    fn intent(bands: Vec<BandIntent>, columns: u32) -> StageIntent {
        StageIntent {
            schema_version: STAGE_INTENT_SCHEMA_VERSION,
            route: "/matters".into(),
            file_key: "KEY".into(),
            node_id: "1:2".into(),
            columns,
            bands,
            authored_by: "a test".into(),
            authored_at: Timestamp::fixed(2026, 8, 3, 9, 0, 0),
            notes: None,
        }
    }

    fn band(band: ReviewRegion, b: BandBox, order: Option<u32>) -> BandIntent {
        BandIntent {
            band,
            box_of: Some(b),
            panes: vec![],
            paint: None,
            order,
        }
    }

    #[test]
    fn a_layer_name_resolves_to_a_band_by_shape_and_not_by_exact_spelling() {
        assert_eq!(band_of("header"), Some(ReviewRegion::Header));
        assert_eq!(band_of("Header"), Some(ReviewRegion::Header));
        assert_eq!(band_of(" Body Rows "), Some(ReviewRegion::BodyRows));
        assert_eq!(band_of("body-rows"), Some(ReviewRegion::BodyRows));
        assert_eq!(
            band_of("Frame 214"),
            None,
            "167 of 188 bands in the subject are literally called Frame, and a layer the closed \
             vocabulary does not name is out of every operation's reach"
        );
    }

    #[test]
    fn a_frame_the_capture_does_not_carry_reads_as_none_and_never_as_empty() {
        let body = capture(
            "1:2",
            &[("header", "9:1", box_of(0.0, 0.0, 1400.0, 48.0), None)],
        );
        assert!(read_frame(&body, "1:2").unwrap().is_some());
        assert!(
            read_frame(&body, "9:9").unwrap().is_none(),
            "a caller that cannot see a frame must say so: a diff against nothing emits a create \
             for every band and rebuilds a frame that was already right"
        );
        // Both of Figma's spellings resolve to the same node.
        assert!(read_frame(&body, "1-2").unwrap().is_some());
    }

    #[test]
    fn a_layer_outside_the_closed_vocabulary_is_recorded_and_unreachable() {
        let body = capture(
            "1:2",
            &[
                ("header", "9:1", box_of(0.0, 0.0, 1400.0, 48.0), None),
                (
                    "a designer's annotation",
                    "9:2",
                    box_of(0.0, 60.0, 200.0, 40.0),
                    None,
                ),
            ],
        );
        let reading = read_frame(&body, "1:2").unwrap().unwrap();
        assert_eq!(reading.bands.len(), 1);
        assert_eq!(
            reading.untouched,
            vec!["a designer's annotation".to_string()]
        );

        // The intent declares only the header. The annotation is not deleted,
        // not moved and not named: it is not a band.
        let operations = diff(&DiffInputs {
            intent: &intent(
                vec![band(
                    ReviewRegion::Header,
                    box_of(0.0, 0.0, 1400.0, 48.0),
                    None,
                )],
                1,
            ),
            reading: &reading,
            paints: &BTreeMap::new(),
        });
        assert!(
            operations.is_empty(),
            "a node VDS did not create and the intent does not name is never touched: \
             {operations:?}"
        );
    }

    /// IDEMPOTENCE NEEDS A TEST, NOT A COMMENT.
    #[test]
    fn applying_the_same_intent_twice_emits_zero_operations_the_second_time() {
        let wanted = intent(
            vec![
                band(
                    ReviewRegion::Header,
                    box_of(0.0, 0.0, 1400.0, 48.0),
                    Some(0),
                ),
                band(
                    ReviewRegion::BodyRows,
                    box_of(0.0, 48.0, 1400.0, 824.0),
                    Some(1),
                ),
            ],
            1,
        );

        // Run one: the frame draws nothing yet.
        let empty = read_frame(&capture("1:2", &[]), "1:2").unwrap().unwrap();
        let first = diff(&DiffInputs {
            intent: &wanted,
            reading: &empty,
            paints: &BTreeMap::new(),
        });
        assert_eq!(first.len(), 2, "{first:?}");
        assert!(
            first
                .iter()
                .all(|o| matches!(o, StageOperation::CreateBand { .. }))
        );

        // Run two, against the frame the first run would have produced.
        let after = read_frame(
            &capture(
                "1:2",
                &[
                    ("header", "9:1", box_of(0.0, 0.0, 1400.0, 48.0), None),
                    ("body_rows", "9:2", box_of(0.0, 48.0, 1400.0, 824.0), None),
                ],
            ),
            "1:2",
        )
        .unwrap()
        .unwrap();
        let second = diff(&DiffInputs {
            intent: &wanted,
            reading: &after,
            paints: &BTreeMap::new(),
        });
        assert!(
            second.is_empty(),
            "the second run must emit nothing, and it is MEASURED here rather than asserted in a \
             comment: {second:?}"
        );
    }

    /// AND THE BAND-NAME KEYING IS WHAT DOES IT.
    ///
    /// Every node id differs from the run that created them, which is what
    /// happens when a node is recreated. A node-keyed diff would report every
    /// band missing and recreate all of them, destroying whatever a designer
    /// had put inside.
    #[test]
    fn a_frame_whose_node_ids_all_changed_still_emits_zero_operations() {
        let wanted = intent(
            vec![
                band(
                    ReviewRegion::Header,
                    box_of(0.0, 0.0, 1400.0, 48.0),
                    Some(0),
                ),
                band(
                    ReviewRegion::BodyRows,
                    box_of(0.0, 48.0, 1400.0, 824.0),
                    Some(1),
                ),
            ],
            1,
        );
        let renumbered = read_frame(
            &capture(
                "1:2",
                &[
                    (
                        "header",
                        "88888:77777",
                        box_of(0.0, 0.0, 1400.0, 48.0),
                        None,
                    ),
                    (
                        "body_rows",
                        "99999:66666",
                        box_of(0.0, 48.0, 1400.0, 824.0),
                        None,
                    ),
                ],
            ),
            "1:2",
        )
        .unwrap()
        .unwrap();
        assert_eq!(renumbered.bands[0].node_id, "88888:77777");
        let operations = diff(&DiffInputs {
            intent: &wanted,
            reading: &renumbered,
            paints: &BTreeMap::new(),
        });
        assert!(
            operations.is_empty(),
            "the diff must not depend on a node id: keyed on one, the second run recreates every \
             band. {operations:?}"
        );
    }

    #[test]
    fn a_variant_spelling_keys_to_the_same_band_and_is_moved_onto_the_canonical_one() {
        let wanted = intent(
            vec![band(
                ReviewRegion::BodyRows,
                box_of(0.0, 48.0, 1400.0, 824.0),
                None,
            )],
            1,
        );
        let reading = read_frame(
            &capture(
                "1:2",
                &[("Body Rows", "9:2", box_of(0.0, 48.0, 1400.0, 824.0), None)],
            ),
            "1:2",
        )
        .unwrap()
        .unwrap();
        let operations = diff(&DiffInputs {
            intent: &wanted,
            reading: &reading,
            paints: &BTreeMap::new(),
        });
        assert_eq!(
            operations,
            vec![StageOperation::SetName {
                band: ReviewRegion::BodyRows,
                to: "body_rows".into()
            }],
            "the variant spelling must key to the same band (so no create is emitted) and be \
             moved onto the canonical one"
        );
    }

    #[test]
    fn only_declared_fields_are_compared_so_an_undeclared_one_is_never_written() {
        // The intent declares the band and NO box. The frame's box is left
        // alone, and nothing is emitted.
        let wanted = intent(
            vec![BandIntent {
                band: ReviewRegion::Footer,
                box_of: None,
                panes: vec![],
                paint: None,
                order: None,
            }],
            1,
        );
        let reading = read_frame(
            &capture(
                "1:2",
                &[(
                    "footer",
                    "9:3",
                    box_of(0.0, 872.0, 1400.0, 28.0),
                    Some("#748eaf"),
                )],
            ),
            "1:2",
        )
        .unwrap()
        .unwrap();
        let operations = diff(&DiffInputs {
            intent: &wanted,
            reading: &reading,
            paints: &BTreeMap::new(),
        });
        assert!(
            operations.is_empty(),
            "an intent binds only for what it declares: {operations:?}"
        );
    }

    #[test]
    fn a_band_the_intent_no_longer_declares_is_the_only_thing_a_delete_reaches() {
        let wanted = intent(
            vec![band(
                ReviewRegion::Header,
                box_of(0.0, 0.0, 1400.0, 48.0),
                None,
            )],
            1,
        );
        let reading = read_frame(
            &capture(
                "1:2",
                &[
                    ("header", "9:1", box_of(0.0, 0.0, 1400.0, 48.0), None),
                    ("rail", "9:4", box_of(0.0, 48.0, 56.0, 824.0), None),
                    (
                        "a hand-drawn note",
                        "9:5",
                        box_of(60.0, 60.0, 200.0, 40.0),
                        None,
                    ),
                ],
            ),
            "1:2",
        )
        .unwrap()
        .unwrap();
        let operations = diff(&DiffInputs {
            intent: &wanted,
            reading: &reading,
            paints: &BTreeMap::new(),
        });
        assert_eq!(
            operations,
            vec![StageOperation::DeleteBand {
                band: ReviewRegion::Rail
            }],
            "the rail is a closed-vocabulary band the intent no longer declares; the note is not \
             a band at all and no operation may reach it"
        );
    }

    #[test]
    fn a_paint_is_written_only_where_the_resolved_value_differs() {
        let mut paints = BTreeMap::new();
        paints.insert(
            "border-control".to_owned(),
            vds_css::colour::parse("#748eaf").unwrap(),
        );
        let wanted = intent(
            vec![BandIntent {
                band: ReviewRegion::Rail,
                box_of: None,
                panes: vec![],
                paint: Some(PaintIntent {
                    property: "--border-control".into(),
                    role: vds_core::FloorScope::ControlBoundary,
                    backdrop: "--surface".into(),
                }),
                order: None,
            }],
            1,
        );

        let matching = read_frame(
            &capture(
                "1:2",
                &[(
                    "rail",
                    "9:4",
                    box_of(0.0, 48.0, 56.0, 824.0),
                    Some("#748eaf"),
                )],
            ),
            "1:2",
        )
        .unwrap()
        .unwrap();
        assert!(
            diff(&DiffInputs {
                intent: &wanted,
                reading: &matching,
                paints: &paints
            })
            .is_empty()
        );

        let differing = read_frame(
            &capture(
                "1:2",
                &[(
                    "rail",
                    "9:4",
                    box_of(0.0, 48.0, 56.0, 824.0),
                    Some("#2b2b2b"),
                )],
            ),
            "1:2",
        )
        .unwrap()
        .unwrap();
        let operations = diff(&DiffInputs {
            intent: &wanted,
            reading: &differing,
            paints: &paints,
        });
        assert_eq!(operations.len(), 1, "{operations:?}");
        assert!(matches!(operations[0], StageOperation::SetPaint { .. }));
    }

    #[test]
    fn a_plan_is_emitted_in_ordered_digest_pinned_chunks() {
        let bands: Vec<BandIntent> = ReviewRegion::ALL
            .into_iter()
            .enumerate()
            .map(|(i, region)| {
                band(
                    region,
                    box_of(0.0, f64::from(i as u32) * 100.0, 1400.0, 90.0),
                    Some(i as u32),
                )
            })
            .collect();
        let wanted = intent(bands, 1);
        let reading = read_frame(&capture("1:2", &[]), "1:2").unwrap().unwrap();
        let operations = diff(&DiffInputs {
            intent: &wanted,
            reading: &reading,
            paints: &BTreeMap::new(),
        });
        assert_eq!(operations.len(), 7);

        let plan = emit_plan(
            &StageId::parse("STG-0001").unwrap(),
            &wanted,
            Digest::of_text("intent"),
            &reading,
            "design/captures/matters.json",
            Digest::of_text("capture"),
            operations,
            Timestamp::fixed(2026, 8, 3, 10, 0, 0),
        )
        .unwrap();
        assert_eq!(plan.operation_count(), 7);
        assert!(plan.untrustworthy_because().unwrap().is_none());
        for (index, chunk) in plan.chunks.iter().enumerate() {
            assert_eq!(chunk.ordinal, index as u32 + 1, "chunks are ordered from 1");
        }
        const {
            assert!(
                CHUNK_CHARACTER_BUDGET < BRIDGE_CODE_LIMIT,
                "the chunk budget is a fraction of the bridge's cap, because what crosses is a \
                 program carrying the operations rather than the operations themselves"
            )
        };
    }

    #[test]
    fn a_plan_larger_than_one_chunk_is_split_and_every_operation_survives() {
        let mut operations = Vec::new();
        for i in 0..400u32 {
            operations.push(StageOperation::SetBox {
                band: ReviewRegion::ALL[(i % 7) as usize],
                box_of: box_of(f64::from(i), 0.0, 100.0, 100.0),
            });
        }
        let wanted = intent(vec![], 1);
        let reading = read_frame(&capture("1:2", &[]), "1:2").unwrap().unwrap();
        let plan = emit_plan(
            &StageId::parse("STG-0001").unwrap(),
            &wanted,
            Digest::of_text("intent"),
            &reading,
            "design/captures/matters.json",
            Digest::of_text("capture"),
            operations,
            Timestamp::fixed(2026, 8, 3, 10, 0, 0),
        )
        .unwrap();
        assert!(
            plan.chunks.len() > 1,
            "400 operations must not fit one chunk"
        );
        assert_eq!(
            plan.operation_count(),
            400,
            "chunking must not lose an operation: a plan quietly narrower than the diff is a \
             partial apply nobody asked for"
        );
        assert!(plan.untrustworthy_because().unwrap().is_none());
    }

    /// G3's input: the SAME derivation, over boxes that do not exist yet.
    #[test]
    fn the_synthetic_document_derives_the_column_count_the_boxes_will_produce() {
        let config = vds_core::ScreensConfig::default();
        let three = intent(
            vec![BandIntent {
                band: ReviewRegion::BodyRows,
                box_of: Some(box_of(0.0, 48.0, 1344.0, 824.0)),
                panes: vec![
                    box_of(0.0, 48.0, 400.0, 800.0),
                    box_of(440.0, 48.0, 400.0, 800.0),
                    box_of(880.0, 48.0, 400.0, 800.0),
                ],
                paint: None,
                order: None,
            }],
            3,
        );
        let (columns, truncated) = crate::frames::columns_of(&synthetic_document(&three), &config);
        assert_eq!(
            columns, 3,
            "the derivation must read the panes as it will after the write"
        );
        assert!(
            !truncated,
            "the synthetic tree is deep enough not to read its own boundary"
        );

        let one = intent(
            vec![BandIntent {
                band: ReviewRegion::BodyRows,
                box_of: Some(box_of(0.0, 48.0, 1344.0, 824.0)),
                panes: vec![box_of(0.0, 48.0, 1344.0, 800.0)],
                paint: None,
                order: None,
            }],
            1,
        );
        let (columns, _) = crate::frames::columns_of(&synthetic_document(&one), &config);
        assert_eq!(
            columns, 1,
            "a screen with no split still has ONE column; 0 is the value that makes a \
             requirement unfailable"
        );
    }
}
