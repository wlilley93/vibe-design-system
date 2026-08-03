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
//!
//! # THE BANDS ARE NOT ALWAYS THE FRAME'S OWN CHILDREN
//!
//! This reader used to take the frame's DIRECT CHILDREN as its bands. On a frame
//! whose bands sit under a named authority layer - `CURRENT SOURCE · /matters` and
//! the other spellings `[screens] authority_markers` carries - it therefore saw no
//! bands at all, reported every declared band missing, and emitted a CREATE for
//! each one. The apply then drew A SECOND FULL SET OF BANDS beside the first, and
//! every instrument in the path reported success: the diff was right about what it
//! had read, the plan was right about the diff, the apply landed every operation,
//! and the verification re-read the same frame the same wrong way and found the
//! delta empty. About a tenth of the frames on the subject estate have that shape,
//! and the frame ledger has resolved the authority layer since it was written.
//! [`crate::frames::authority_child`] is now the ONE place that precedence lives,
//! and [`FrameReading::bands_under`] says on the reading's face which subtree the
//! bands were read from.
//!
//! # SILENCE IS NOT PERMISSION TO DELETE
//!
//! [`StageOperation::DeleteBand`] is emitted only for a band the intent lists in
//! `deletes`. It used to be emitted for every closed-vocabulary band in the frame
//! that the intent did not declare, which meant an intent that had simply never
//! mentioned a `facets` band deleted the one a designer had drawn - and G2
//! compares the intent to the SCREEN RECORD and never to the canvas, so the one
//! destructive verb in the vocabulary was the only operation emitted with no gate
//! reading behind it. A band present in the frame that the intent neither declares
//! nor deletes is reported by [`FrameReading::undeclared_bands`] and left alone.

use std::collections::{BTreeMap, BTreeSet};

use vds_core::{
    BandBox, Digest, GateVerdict, PlanChunk, Result, ReviewRegion, ScreensConfig, StageContainer,
    StageId, StageIntent, StageOperation, StagePlan, Timestamp, VdsError,
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
    /// The exact subtree whose direct children were read as bands. For an
    /// unlabelled frame this is the frame itself; for a named current-source
    /// layer it is that layer. Every emitted operation carries the same scope.
    pub container: StageContainer,
    /// The NAME of the authority layer the bands were read from, or `None` where
    /// the frame's own children are the bands.
    ///
    /// On the reading's face because it changes what every operation below is
    /// about. A reader who cannot see which subtree was read cannot tell a frame
    /// with no bands from a frame whose bands were looked for in the wrong place,
    /// and those two produce the same diff and opposite outcomes.
    pub bands_under: Option<String>,
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

    /// The bands this frame DRAWS that an intent neither declares nor deletes.
    ///
    /// Not an operation and not a finding: a fact the plan publishes. These are
    /// the bands the old diff deleted on the strength of the intent's silence.
    pub fn undeclared_bands(&self, intent: &StageIntent) -> Vec<ReviewRegion> {
        let declared: BTreeSet<ReviewRegion> = intent
            .declared_bands()
            .into_iter()
            .chain(intent.declared_deletes())
            .collect();
        self.bands
            .iter()
            .map(|state| state.band)
            .filter(|band| !declared.contains(band))
            .collect()
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
///
/// `config` is the project's own [`ScreensConfig`], and it is a parameter rather
/// than a default because the words a file uses for "this layer is the current
/// source" are the SUBJECT's vocabulary and live in `[screens]
/// authority_markers`. A default here would resolve the authority layer only for
/// projects that happen to use the shipped spellings, and fail silently - by
/// creating a second set of bands - for every project that does not.
pub fn read_frame(
    body: &str,
    node_id: &str,
    config: &ScreensConfig,
) -> Result<Option<FrameReading>> {
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
    Ok(Some(reading_of(&wanted, document, config)?))
}

fn reading_of(
    node_id: &str,
    document: &serde_json::Value,
    config: &ScreensConfig,
) -> Result<FrameReading> {
    let frame_box = box_of(document).unwrap_or(BandBox {
        x: 0.0,
        y: 0.0,
        width: 0.0,
        height: 0.0,
    });
    let mut bands = Vec::new();
    let mut untouched = Vec::new();

    // WHERE THE BANDS LIVE. Resolved through the authority layer where the frame
    // names one, by the same precedence the frame ledger uses, because a diff that
    // looked only at the frame's own children saw no bands under a `CURRENT
    // SOURCE` layer and created a second full set beside the ones already drawn.
    let (bands_under, container, scope) = match crate::frames::authority_child(document, config) {
        Some(selected) => {
            let Some(selected_id) = selected.node_id.clone() else {
                return Err(VdsError::precondition(format!(
                    "the authority layer {:?} in frame {} has no node id. A staged operation \
                     cannot be scoped to a name shared by a sibling, so this capture is refused \
                     rather than risking a write to the wrong subtree.",
                    selected.name, node_id
                )));
            };
            (
                Some(selected.name.clone()),
                selected.document,
                StageContainer {
                    node_id: selected_id,
                    name: selected.name,
                },
            )
        }
        None => (
            None,
            document,
            StageContainer {
                node_id: node_id.to_owned(),
                name: document
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_owned(),
            },
        ),
    };
    // The authority layer's SIBLINGS. Out of every operation's reach - a delete
    // reaches one band inside the resolved container and nothing else - and
    // recorded, because "this frame also carries a legacy underlay" is exactly the
    // fact a reviewer needs in order to not read the plan as covering it.
    if bands_under.is_some() {
        for child in document
            .get("children")
            .and_then(|v| v.as_array())
            .map(Vec::as_slice)
            .unwrap_or_default()
        {
            let name = child
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            if Some(name) != bands_under.as_deref() {
                untouched.push(name.to_owned());
            }
        }
    }

    let children = container
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
    Ok(FrameReading {
        node_id: node_id.to_owned(),
        frame_name: document
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_owned(),
        frame_box,
        container: scope,
        bands_under,
        bands,
        untouched,
    })
}

/// Why the frame this capture draws is not the frame the intent declares, or
/// `None`.
///
/// THE LIMB THAT STOPS THE DECLARATION BEING TYPED. G3 refuses an extent that is
/// not the canonical shell, and reads it from the intent because no capture is in
/// front of it: `vds stage add` has none and a proof may not fetch one
/// (VDS S-7(2)(1)). So the declaration alone is a claim, and a claim believed is a
/// claim that can be MINTED - which is the same defect `staged_write` R7 exists
/// for, one artefact along. `vds stage plan` HAS a capture, and this is where the
/// claim meets it.
///
/// SIZE ONLY. Where a frame sits on the canvas is not a decision about the screen,
/// and comparing an origin would refuse a frame somebody had merely moved.
pub fn extent_disagreement(intent: &StageIntent, reading: &FrameReading) -> Option<String> {
    let declared = intent.frame_extent;
    if same(declared.width, reading.frame_box.width)
        && same(declared.height, reading.frame_box.height)
    {
        return None;
    }
    let narrower = reading.frame_box.width < declared.width;
    let shorter = reading.frame_box.height < declared.height;
    Some(format!(
        "the intent declares a target frame and the capture draws {} that is {} in width and {} \
         in height. Neither number is repeated here: they are lengths, and a finding lands under \
         the tree `no_stored_values` scans. The declared extent is what G3 measured the canonical \
         shell against, so if the capture is right then the gate cleared a frame it never saw, \
         and if the intent is right then this capture is of another frame. Open node {} and \
         settle which.",
        reading.frame_name,
        if narrower { "SMALLER" } else { "LARGER" },
        if shorter { "SMALLER" } else { "LARGER" },
        reading.node_id
    ))
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
    let container = reading.container.clone();
    let mut out = Vec::new();

    // DELETE. Two conditions, checked together and never separately: the band's
    // name is in the closed vocabulary (so it is a band at all), and the intent
    // NAMES IT in `deletes`. Anything else in the frame is in `untouched` or in
    // `undeclared_bands` and no operation here can reach it. There is no
    // page-level and no frame-level delete, and the reason is in the enum's own
    // doc: the 2026-07-25 loss came from a delete-page-and-recreate step, and
    // widening the vocabulary makes that loss repeatable through the sanctioned
    // path.
    //
    // SILENCE IS NOT PERMISSION TO DELETE, and this limb used to read the other
    // way round: every closed-vocabulary band the intent did not declare was
    // deleted, so an intent about the header removed a `facets` band a designer
    // had drawn by never mentioning it. Only a band the frame ACTUALLY DRAWS is
    // emitted for, because a delete for a band that is not there is an operation
    // the apply cannot land and the verification would then report as residual
    // forever.
    for band in intent.declared_deletes() {
        if reading.band(band).is_some() {
            out.push(StageOperation::DeleteBand {
                band,
                container: container.clone(),
            });
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
                container: container.clone(),
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
                container: container.clone(),
            });
        }

        if let Some(box_of) = band.box_of
            && let Some(state) = existing
            && !same_box(&state.box_of, &box_of)
        {
            out.push(StageOperation::SetBox {
                band: band.band,
                box_of,
                container: container.clone(),
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
                    container: container.clone(),
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
                container: container.clone(),
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
///
/// `gates` are the readings this plan is emitted UNDER, and they are a parameter
/// rather than something read here because the gates are a proof-crate concern
/// and this crate must not depend on it. A plan carried no reading from any gate
/// at all until this argument existed: the artefact the whole capability calls
/// REVIEWABLE could not tell a reviewer whether a single gate had run over the
/// operations beneath it. [`StagePlan::coverage`] is derived from them here, so
/// the line and the readings cannot be emitted disagreeing.
#[allow(clippy::too_many_arguments)]
pub fn emit_plan(
    stage: &StageId,
    intent: &StageIntent,
    intent_digest: Digest,
    reading: &FrameReading,
    reading_path: &str,
    reading_digest: Digest,
    operations: Vec<StageOperation>,
    gates: Vec<GateVerdict>,
    at: Timestamp,
) -> Result<StagePlan> {
    let container = reading.container.clone();
    if let Some(operation) = operations
        .iter()
        .find(|operation| operation.container() != &container)
    {
        return Err(VdsError::precondition(format!(
            "{} {} is scoped to {:?}, but the reading selected {:?}. A plan cannot hand an \
             apply operation a parent other than the resolved authority container.",
            operation.verb(),
            operation.band(),
            operation.container(),
            container
        )));
    }
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
        container,
        untouched: reading.untouched.clone(),
        gates,
        coverage: String::new(),
        chunks,
        content_digest: Digest::of_text("placeholder"),
    };
    // DERIVED from the readings above and never passed in beside them. The
    // coverage line is the sentence a reviewer reads instead of counting four
    // readings, so it is computed from the only copy of them that exists.
    plan.coverage = plan.gate_coverage_line();
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
/// THE ROOT IS THE INTENT'S DECLARED EXTENT, not [`SHELL_WIDTH`] by
/// [`SHELL_HEIGHT`]. It used to be the shell unconditionally, which put a second
/// answer to "how big is the frame this write is aimed at" one function away from
/// the intent's own: 80 of 188 frames on the estate this was written for are the
/// body with no shell around it, and a derivation run inside a root larger than
/// the real frame is a derivation about a frame nobody drew. G3 refuses an extent
/// that is not the canonical shell before it ever gets here, so on a lawful intent
/// the two agree - and agreeing by derivation is the point, because the alternative
/// is two constants that agree until one of them is edited.
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
            width: intent.frame_extent.width,
            height: intent.frame_extent.height,
        },
        serde_json::Value::Array(bands),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use vds_core::{
        BandIntent, FrameExtent, GateReading, PaintIntent, ReviewRegion,
        STAGE_INTENT_SCHEMA_VERSION, StageContainer, StageGate,
    };

    /// The project's own vocabulary, which is what decides whether a layer name
    /// says "this subtree is the current source".
    fn config() -> ScreensConfig {
        ScreensConfig::default()
    }

    /// Every gate cleared. The plan's readings are not the subject of any test in
    /// this module, so they are supplied rather than measured here; what IS
    /// measured here is that the plan publishes them at all.
    fn all_cleared() -> Vec<GateVerdict> {
        StageGate::ALL
            .into_iter()
            .map(|gate| GateVerdict {
                gate,
                reading: GateReading::Cleared,
                because: "measured elsewhere; supplied by this test".into(),
            })
            .collect()
    }

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
            frame_extent: FrameExtent {
                width: SHELL_WIDTH,
                height: SHELL_HEIGHT,
            },
            columns,
            bands,
            deletes: vec![],
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
        assert!(read_frame(&body, "1:2", &config()).unwrap().is_some());
        assert!(
            read_frame(&body, "9:9", &config()).unwrap().is_none(),
            "a caller that cannot see a frame must say so: a diff against nothing emits a create \
             for every band and rebuilds a frame that was already right"
        );
        // Both of Figma's spellings resolve to the same node.
        assert!(read_frame(&body, "1-2", &config()).unwrap().is_some());
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
        let reading = read_frame(&body, "1:2", &config()).unwrap().unwrap();
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
        let empty = read_frame(&capture("1:2", &[]), "1:2", &config())
            .unwrap()
            .unwrap();
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
            &config(),
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
            &config(),
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
            &config(),
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
                to: "body_rows".into(),
                container: reading.container.clone(),
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
            &config(),
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

    /// SILENCE IS NOT PERMISSION TO DELETE, and this is the test that says so.
    ///
    /// The frame draws a `rail` the intent does not mention and a hand-drawn note
    /// that is not a band at all. The old diff deleted the rail on the strength of
    /// the intent's silence; nothing may now reach either of them, and the rail is
    /// reported instead.
    #[test]
    fn a_band_the_intent_does_not_mention_is_left_alone_and_reported() {
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
            &config(),
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
            "the intent says nothing about the rail, and silence is not permission to delete: \
             {operations:?}"
        );
        assert_eq!(
            reading.undeclared_bands(&wanted),
            vec![ReviewRegion::Rail],
            "the rail is a band the frame draws and the intent neither declares nor deletes, and \
             the plan publishes it rather than acting on it"
        );
        assert_eq!(
            reading.untouched,
            vec!["a hand-drawn note".to_string()],
            "the note is not a band at all and no operation may reach it"
        );

        // AND THE DELETE IS STILL REACHABLE, or the assertion above would be
        // satisfied by a diff that had simply lost the verb. Naming the rail in
        // `deletes` emits exactly one operation, over exactly that band.
        let mut asked = wanted.clone();
        asked.deletes = vec![ReviewRegion::Rail];
        assert_eq!(
            diff(&DiffInputs {
                intent: &asked,
                reading: &reading,
                paints: &BTreeMap::new(),
            }),
            vec![StageOperation::DeleteBand {
                band: ReviewRegion::Rail,
                container: reading.container.clone(),
            }],
            "an EXPLICIT delete must still reach the band it names"
        );

        // A delete naming a band the frame does not draw emits nothing: an
        // operation the apply cannot land would be reported as residual forever
        // and the verification could never declare success.
        let mut absent = wanted.clone();
        absent.deletes = vec![ReviewRegion::Footer];
        assert!(
            diff(&DiffInputs {
                intent: &absent,
                reading: &reading,
                paints: &BTreeMap::new(),
            })
            .is_empty(),
            "a delete for a band that is not there is an operation nothing can land"
        );
    }

    /// THE HIGH DEFECT, SEEDED. The diff read the frame's DIRECT CHILDREN while
    /// `frames.rs` had resolved a named CURRENT SOURCE authority layer since it was
    /// written, so on about a tenth of the estate's frames the diff saw ZERO bands,
    /// created a full SECOND set beside the ones already drawn, and every instrument
    /// reported success - the verification re-read the frame the same wrong way and
    /// found no residual.
    ///
    /// # The negative control is the load-bearing half
    ///
    /// A zero-operations assertion over a fixture the NAIVE reading also handles
    /// proves nothing at all: it would pass on the broken code. So the naive
    /// reading is reproduced here, over the same bytes, and asserted to find ZERO
    /// bands first. Only then does the zero-operations result mean the authority
    /// layer was resolved.
    #[test]
    fn bands_under_a_current_source_layer_are_found_and_the_naive_reading_finds_none() {
        let leaf = |name: &str, b: BandBox| {
            serde_json::json!({
                "id": "9:9",
                "name": name,
                "type": "FRAME",
                "absoluteBoundingBox": {"x": b.x, "y": b.y, "width": b.width, "height": b.height},
                "children": [],
            })
        };
        let header = box_of(0.0, 0.0, 1400.0, 48.0);
        let body = box_of(0.0, 48.0, 1400.0, 824.0);
        let authority = serde_json::json!({
            "id": "9:100",
            "name": "CURRENT SOURCE · /matters",
            "type": "FRAME",
            "absoluteBoundingBox": {"x": 0.0, "y": 0.0, "width": 1400.0, "height": 900.0},
            "children": [leaf("header", header), leaf("body_rows", body)],
        });
        let underlay = serde_json::json!({
            "id": "9:200",
            "name": "LEGACY UNDERLAY · body",
            "type": "FRAME",
            "absoluteBoundingBox": {"x": 0.0, "y": 0.0, "width": 1400.0, "height": 900.0},
            // The sibling deliberately carries a band with the SAME name as a
            // current-source band. A delete must still bind to the selected
            // authority node, not search the whole frame by band name.
            "children": [leaf("header", header)],
        });
        let reference = serde_json::json!({
            "id": "9:300",
            "name": "REFERENCE · /matters",
            "type": "FRAME",
            "absoluteBoundingBox": {"x": 0.0, "y": 0.0, "width": 1400.0, "height": 900.0},
            "children": [leaf("body_rows", body)],
        });
        let document = serde_json::json!({
            "id": "1:2",
            "name": "Screen · /matters",
            "type": "FRAME",
            "absoluteBoundingBox": {"x": 0.0, "y": 0.0, "width": SHELL_WIDTH, "height": SHELL_HEIGHT},
            "children": [underlay, reference, authority],
        });
        let body_text =
            serde_json::json!({"nodes": {"1:2": {"document": document.clone()}}}).to_string();

        // ---- THE NEGATIVE CONTROL. The reading this code used to take: the
        // frame's own children, keyed through the same closed vocabulary. It must
        // find NOTHING, or every assertion below is satisfied by the broken code
        // as well as by the fixed one.
        let naive: Vec<ReviewRegion> = document["children"]
            .as_array()
            .expect("the frame's own children")
            .iter()
            .filter_map(|child| band_of(child["name"].as_str().unwrap_or_default()))
            .collect();
        assert!(
            naive.is_empty(),
            "the negative control is broken: the naive direct-children reading finds {naive:?} on \
             this fixture, so a zero-operations assertion over it would pass on the defect too"
        );

        // ---- THE READING. Resolved through the authority layer, and it SAYS SO.
        let reading = read_frame(&body_text, "1:2", &config()).unwrap().unwrap();
        assert_eq!(
            reading.bands_under.as_deref(),
            Some("CURRENT SOURCE · /matters"),
            "the reading must publish which subtree its bands came from"
        );
        assert_eq!(reading.bands.len(), 2, "{:?}", reading.bands);
        assert_eq!(
            reading.untouched,
            vec![
                "LEGACY UNDERLAY · body".to_string(),
                "REFERENCE · /matters".to_string(),
            ],
            "the authority layer's siblings are out of every operation's reach and recorded"
        );
        assert_eq!(
            reading.container,
            StageContainer {
                node_id: "9:100".into(),
                name: "CURRENT SOURCE · /matters".into(),
            }
        );

        // ---- THE OUTCOME THE DEFECT PRODUCED. An intent that is exactly what the
        // authority layer already draws emits NOTHING. Under the direct-children
        // reading this emitted a create per band and drew a second full set.
        let wanted = intent(
            vec![
                band(ReviewRegion::Header, header, Some(0)),
                band(ReviewRegion::BodyRows, body, Some(1)),
            ],
            1,
        );
        let operations = diff(&DiffInputs {
            intent: &wanted,
            reading: &reading,
            paints: &BTreeMap::new(),
        });
        assert!(
            operations.is_empty(),
            "the frame already draws this intent under its authority layer, so the diff must be \
             empty. The naive reading emitted a CREATE for each of these and the apply drew a \
             second full set of bands: {operations:?}"
        );

        // The destructive arm is also pinned to that same container. The
        // legacy sibling carries a header too, so a bridge that ignores the
        // scope could delete the wrong one while still receiving a lawful
        // `delete-band header` verb.
        let delete_only = intent(vec![], 1);
        let mut delete_only = delete_only;
        delete_only.deletes = vec![ReviewRegion::Header];
        let deletes = diff(&DiffInputs {
            intent: &delete_only,
            reading: &reading,
            paints: &BTreeMap::new(),
        });
        assert_eq!(deletes.len(), 1, "{deletes:?}");
        assert_eq!(deletes[0].container(), &reading.container);
        assert_eq!(deletes[0].band(), ReviewRegion::Header);
    }

    /// A named authority layer that is genuinely empty is still the selected
    /// container. It emits creates under that layer; it must not fall back to
    /// the frame's direct children or report an empty frame as the authority.
    #[test]
    fn an_empty_current_source_layer_emits_creates_in_that_layer() {
        let header = box_of(0.0, 0.0, 1400.0, 48.0);
        let body = box_of(0.0, 48.0, 1400.0, 824.0);
        let document = serde_json::json!({
            "id": "1:2",
            "name": "Screen · /matters",
            "type": "FRAME",
            "absoluteBoundingBox": {"x": 0.0, "y": 0.0, "width": SHELL_WIDTH, "height": SHELL_HEIGHT},
            "children": [{
                "id": "9:8",
                "name": "LEGACY UNDERLAY · body",
                "type": "FRAME",
                "absoluteBoundingBox": {"x": 0.0, "y": 0.0, "width": SHELL_WIDTH, "height": SHELL_HEIGHT},
                "children": [{"id": "8:1", "name": "header", "absoluteBoundingBox": {"x": 0.0, "y": 0.0, "width": 1400.0, "height": 48.0}}]
            }, {
                "id": "9:7",
                "name": "CURRENT SOURCE · /matters",
                "type": "FRAME",
                "absoluteBoundingBox": {"x": 0.0, "y": 0.0, "width": SHELL_WIDTH, "height": SHELL_HEIGHT},
                "children": []
            }]
        });
        let body_text = serde_json::json!({"nodes": {"1:2": {"document": document}}}).to_string();
        let reading = read_frame(&body_text, "1:2", &config()).unwrap().unwrap();
        assert_eq!(reading.bands.len(), 0);
        assert_eq!(
            reading.bands_under.as_deref(),
            Some("CURRENT SOURCE · /matters")
        );
        assert_eq!(reading.container.node_id, "9:7");

        let wanted = intent(
            vec![
                band(ReviewRegion::Header, header, Some(0)),
                band(ReviewRegion::BodyRows, body, Some(1)),
            ],
            1,
        );
        let operations = diff(&DiffInputs {
            intent: &wanted,
            reading: &reading,
            paints: &BTreeMap::new(),
        });
        assert_eq!(operations.len(), 2, "{operations:?}");
        assert!(
            operations.iter().all(|operation| {
                matches!(operation, StageOperation::CreateBand { .. })
                    && operation.container() == &reading.container
            }),
            "{operations:?}"
        );
    }

    /// The marker vocabulary belongs to the project. Both the frame ledger and
    /// the staged reader must resolve a custom marker, or the latter silently
    /// falls back to the frame's direct children and can duplicate the drawing.
    #[test]
    fn the_staged_reader_and_frame_ledger_share_project_authority_markers() {
        let mut custom = config();
        custom.authority_markers = vec!["CANONICAL SOURCE".into()];
        let document = serde_json::json!({
            "id": "1:2",
            "name": "Screen · /matters",
            "type": "FRAME",
            "absoluteBoundingBox": {"x": 0.0, "y": 0.0, "width": SHELL_WIDTH, "height": SHELL_HEIGHT},
            "children": [
                {"id": "9:8", "name": "REFERENCE · old", "children": [{"id": "8:1", "name": "header", "absoluteBoundingBox": {"x": 0.0, "y": 0.0, "width": 1400.0, "height": 48.0}}]},
                {"id": "9:7", "name": "CANONICAL SOURCE · /matters", "children": [{"id": "8:2", "name": "body_rows", "absoluteBoundingBox": {"x": 0.0, "y": 48.0, "width": 1400.0, "height": 824.0}}]}
            ]
        });
        let capture =
            serde_json::json!({"nodes": {"1:2": {"document": document.clone()}}}).to_string();

        let reading = read_frame(&capture, "1:2", &custom).unwrap().unwrap();
        assert_eq!(
            reading.bands_under.as_deref(),
            Some("CANONICAL SOURCE · /matters")
        );
        assert_eq!(reading.bands[0].band, ReviewRegion::BodyRows);

        let ledger = crate::frames::build_ledger("KEY", &[capture], &custom, "a test").unwrap();
        assert_eq!(
            ledger.row("1:2").unwrap().authority_layer,
            "CANONICAL SOURCE · /matters"
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
            &config(),
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
            &config(),
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
        let reading = read_frame(&capture("1:2", &[]), "1:2", &config())
            .unwrap()
            .unwrap();
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
            all_cleared(),
            Timestamp::fixed(2026, 8, 3, 10, 0, 0),
        )
        .unwrap();
        assert_eq!(plan.operation_count(), 7);
        assert!(plan.untrustworthy_because().unwrap().is_none());
        assert_eq!(plan.container, reading.container);
        assert!(
            plan.operations()
                .all(|operation| operation.container() == &plan.container),
            "every plan operation must carry the resolved parent scope"
        );

        // THE PLAN PUBLISHES THE READINGS IT WAS EMITTED UNDER. It carried none at
        // all, so a reviewer holding the artefact this capability calls REVIEWABLE
        // could not see whether one gate had run over the operations below it.
        assert_eq!(plan.gates.len(), StageGate::ALL.len(), "{:?}", plan.gates);
        assert!(plan.gates_not_asked().is_empty());
        assert!(
            plan.coverage.contains("4 of 4 gate(s) CLEARED"),
            "the coverage line must be on the face of the artefact: {:?}",
            plan.coverage
        );
        // And it is DERIVED here rather than passed in beside the readings, so it
        // cannot be emitted disagreeing with them.
        assert_eq!(plan.coverage, plan.gate_coverage_line());

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
        let container = StageContainer {
            node_id: "1:2".into(),
            name: "Screen".into(),
        };
        for i in 0..400u32 {
            operations.push(StageOperation::SetBox {
                band: ReviewRegion::ALL[(i % 7) as usize],
                box_of: box_of(f64::from(i), 0.0, 100.0, 100.0),
                container: container.clone(),
            });
        }
        let wanted = intent(vec![], 1);
        let reading = read_frame(&capture("1:2", &[]), "1:2", &config())
            .unwrap()
            .unwrap();
        let plan = emit_plan(
            &StageId::parse("STG-0001").unwrap(),
            &wanted,
            Digest::of_text("intent"),
            &reading,
            "design/captures/matters.json",
            Digest::of_text("capture"),
            operations,
            all_cleared(),
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
