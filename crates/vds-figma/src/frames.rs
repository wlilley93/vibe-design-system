//! The frame ledger: what the decided-target file's SCREEN frames actually draw.
//!
//! [`crate::ledger`] records the file's COMPONENT sets. This records its SCREEN
//! frames, and it exists because parity is a claim about screens and the ten
//! proof kinds that came before it all read components. A page can render every
//! registered component, each in a registered state, in an arrangement its frame
//! does not draw, and nothing in VDS could see it.
//!
//! # Out of band, from a SAVED response, for the same reason as everything else
//!
//! VDS S-7(2)(1) forbids a network call inside a proof, so this reads a saved
//! `GET /v1/files/:key/nodes` response and never the API, exactly as
//! [`crate::pull::SavedResponse`] does for the file endpoint. It is the `nodes`
//! endpoint and not `files` because a screen file is far too large to fetch
//! whole and because that endpoint is the one that takes a `depth`, which turns
//! out to be load-bearing (see [`FrameRow::truncated`]).
//!
//! # Four findings from the prior art, all of them general
//!
//! **1. A frame is not one drawing, and it says which one governs.** Authority
//! lives in layer NAMES, and the matching rule is asymmetric: an authority
//! marker is matched anywhere and case-insensitively, a quarantine marker only
//! as the leading segment. Both halves were paid for. See [`authority_of`].
//!
//! **2. A frame may DISCLAIM ITSELF.** `LEGACY / TARGET REFERENCE - /sql - NOT
//! SOURCE CURRENT` states no contract, and 25 of 188 frames in the subject did
//! this. Comparing shipped code against one produces a difference that is real
//! and means nothing, which is a gate crying wolf about a route the designer
//! already marked as having no current drawing.
//!
//! **3. An unseen child is not an absent child.** A depth-limited capture
//! records `children: []` with NO truncation flag anywhere in the payload; the
//! only thing that knows is the depth that was asked for. A ledger built from a
//! depth-3 capture stated, as a fact, that one of the two busiest routes in the
//! product drew nothing in its content frame. It has four children. So the
//! depth is recorded on the ledger and every leaf sitting at it is marked, and
//! "the frame draws nothing here" is never again the same value as "we did not
//! look".
//!
//! **4. Geometry is the only vocabulary every frame shares.** Band NAMES are
//! unusable: 167 of 188 bands in the subject are literally called `Frame`, and
//! most of the rest are prose. Columns are therefore clustered by x-interval.
//!
//! # What this ledger holds, and what it refuses to hold
//!
//! A COUNT of columns and a list of region NAMES. Never a width. The prior art
//! records `[924, 420]`, which is the storing form VDS S-2(2) prohibits: this
//! file lands under `.vds/**`, and `no_stored_values` R3 fails on a number
//! carrying a CSS length unit and R7 on a field whose name is a realisation. A
//! ledger that recorded widths would fail the gate forever, on a file VDS wrote
//! itself, with no lawful way back because a record is never deleted.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use vds_core::{Digest, Result, ScreensConfig, Timestamp, VdsError};

pub const LEDGER_SCHEMA_VERSION: u32 = 1;
pub const GENERATOR_COMMAND: &str = "vds figma frames";

// ---------------------------------------------------------------- thresholds
//
// THESE ARE LENGTHS, AND THEY LIVE IN CODE RATHER THAN IN THE CONFIG FOR THAT
// REASON. `.vds/config.toml` is scanned by `no_stored_values`, which fails on a
// number carrying a CSS length unit anywhere under the record (R3), and putting
// a pixel threshold there would invite exactly the field VDS S-2(2) prohibits.
// They are derivation parameters, not design values: nothing here is a decision
// about how a screen should look, and changing one changes what the instrument
// can resolve, not what the design is.

/// A child narrower or shorter than this is furniture, not a column.
///
/// A rule, a divider or an icon sits inside a column and would otherwise be
/// clustered as one.
const MIN_COLUMN_WIDTH: f64 = 120.0;
const MIN_COLUMN_HEIGHT: f64 = 60.0;

/// A band spanning at least this fraction of the content width is a HEADER, and
/// including it merges every column into one because it overlaps them all.
///
/// The single most important number here. Without it, a screen with a page
/// header above two panes reports ONE column, every time, and the gate would
/// have been quietly measuring nothing on the majority of screens.
const FULL_WIDTH_FRACTION: f64 = 0.85;

/// How much two children may overlap and still be separate columns.
///
/// Designers draw hairline rules and shadows across a seam, and without this
/// slack two panes sharing a one-unit border read as one.
const SEAM_SLACK: f64 = 8.0;

/// A band this tall or taller may be the one that CONTAINS the columns.
///
/// The common shape is a header band above a content band, where the columns
/// are the content band's children rather than the bands themselves.
const MIN_CONTENT_BAND_HEIGHT: f64 = 280.0;

// -------------------------------------------------------------------- ledger

/// How a frame's authoritative subtree was chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityBy {
    /// A child layer NAMED itself authoritative, and it governs.
    NamedLayer,
    /// The frame's own children are the shell, so the frame governs.
    FrameOwnChildren,
    /// Nothing in the frame claims authority. The frame governs by default, and
    /// this is recorded rather than smoothed over: a reader is entitled to know
    /// the difference between a frame that SAID it was current and one that
    /// merely was not contradicted.
    Unlabelled,
}

impl AuthorityBy {
    pub fn as_str(self) -> &'static str {
        match self {
            AuthorityBy::NamedLayer => "named_layer",
            AuthorityBy::FrameOwnChildren => "frame_own_children",
            AuthorityBy::Unlabelled => "unlabelled",
        }
    }
}

/// One screen frame, as the decided-target file draws it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameRow {
    /// The node id, normalised to the `12:34` spelling.
    pub node_id: String,
    /// The frame's own name in Figma. A NAME, not a value.
    pub frame_name: String,
    /// The name of the layer that GOVERNS this frame.
    pub authority_layer: String,
    pub authority_by: AuthorityBy,
    /// Every layer this frame carries that is not the authority.
    ///
    /// Recorded rather than dropped: "this route has a legacy underlay" is
    /// exactly the fact a reader needs in order to not build from it.
    #[serde(default)]
    pub quarantined: Vec<String>,
    /// Whether the authoritative layer DISCLAIMS ITSELF, i.e. says in its own
    /// name that it is not source-current or was never built.
    ///
    /// Such a frame states no contract, and comparing anything to it is
    /// meaningless. It is recorded here so the exclusion stays VISIBLE: a
    /// route quietly dropped from a gate is a route scored clean by silence.
    #[serde(default)]
    pub disclaimed: bool,
    /// The shell regions found under the authority, in the order
    /// `[screens] region_names` declares them.
    #[serde(default)]
    pub regions: Vec<String>,
    /// How many side-by-side content PANES the frame draws.
    ///
    /// A count and never a width; see the module note. A frame with no split
    /// draws ONE column, not zero: a screen with no split still has a content
    /// pane, and reporting 0 made the prior art score a route
    /// `frame=0 code=1` and call agreement a deviation.
    pub columns: u32,
    /// Whether any node the column derivation READ sat on the capture boundary.
    ///
    /// The most important field in this ledger. A Figma response carries no
    /// "this subtree was cut off" flag, so a childless node at the capture depth
    /// and a genuinely empty one are the same bytes. Where this is true the
    /// count above is a reading of a truncated tree, and `screen_parity` refuses
    /// to enforce the row rather than reporting a number it cannot stand behind.
    #[serde(default)]
    pub truncated: bool,
}

/// A frame in the capture that no screen record claims.
///
/// The other direction of the same question the component ledger asks with
/// `unclaimed` (`crates/vds-figma/src/ledger.rs:67`): a screen drawn in the
/// decided-target file and absent from the screen register is one design has
/// committed to and governance has never seen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameLedger {
    pub schema_version: u32,
    /// Excluded from `content_digest`, so re-generating from an unchanged
    /// capture does not move a digest any proof cites.
    pub generated_at: Timestamp,
    pub generated_by: String,
    pub file_key: String,
    /// The deepest chain present in the capture.
    ///
    /// DERIVED from the response and never taken from a flag the caller passed,
    /// and the direction of the error is why. If the capture went deeper than
    /// the deepest real chain, this number comes out smaller and marks genuine
    /// leaves as truncated, which costs enforcement on a few rows and reports
    /// nothing false. A caller-asserted depth that was too large would do the
    /// opposite: it would certify a truncated subtree as fully seen, which is
    /// the exact error this field exists to prevent.
    pub capture_depth: u32,
    /// How many leaves in the whole capture sit at the boundary.
    pub truncated_leaves: u32,
    pub content_digest: Digest,
    pub frames: Vec<FrameRow>,
    /// What the generator could not see, in the words a reader needs.
    #[serde(default)]
    pub notes: Vec<String>,
}

impl FrameLedger {
    pub fn compute_content_digest(&self) -> Result<Digest> {
        #[derive(Serialize)]
        struct Content<'a> {
            schema_version: u32,
            generated_by: &'a str,
            file_key: &'a str,
            capture_depth: u32,
            truncated_leaves: u32,
            frames: &'a [FrameRow],
        }
        Digest::of_value(&Content {
            schema_version: self.schema_version,
            generated_by: &self.generated_by,
            file_key: &self.file_key,
            capture_depth: self.capture_depth,
            truncated_leaves: self.truncated_leaves,
            frames: &self.frames,
        })
    }

    /// The row for a node id, in either of Figma's two spellings.
    pub fn row(&self, node_id: &str) -> Option<&FrameRow> {
        let key = normalise_node_id(node_id);
        self.frames.iter().find(|f| f.node_id == key)
    }
}

/// Why the frame ledger cannot be relied on.
///
/// The same three limbs [`crate::ledger::check_fresh`] settles, and the same
/// fourth it cannot: whether the decided-target file has changed since the
/// capture is a network read, and VDS S-7(2)(1) forbids one inside a proof.
pub fn check_fresh(ledger: &FrameLedger, expected_file_key: Option<&str>) -> Result<()> {
    if ledger.schema_version > LEDGER_SCHEMA_VERSION {
        return Err(VdsError::SchemaVersionAhead {
            path: "the frame ledger".into(),
            kind: "frame ledger",
            found: ledger.schema_version,
            understood: LEDGER_SCHEMA_VERSION,
        });
    }
    if ledger.content_digest != ledger.compute_content_digest()? {
        return Err(VdsError::precondition(
            "the frame ledger's content digest does not match its own contents, so it was \
             edited after it was generated. A ledger is a generated inventory and never \
             hand-edited (VDS S-4(2)).\n  Regenerate with: vds figma frames --from <capture>",
        ));
    }
    if let Some(expected) = expected_file_key
        && ledger.file_key != expected
    {
        return Err(VdsError::precondition(format!(
            "the frame ledger was captured from file {:?} and the screen register names {:?}. \
             Two decided-target files is two opinions about what is decided.\n  \
             Regenerate with: vds figma frames --from <capture>",
            ledger.file_key, expected
        )));
    }
    Ok(())
}

// ------------------------------------------------------------------ building

/// One node of the capture, with the two things a raw payload cannot say.
#[derive(Debug, Clone)]
struct Node {
    name: String,
    visible: bool,
    box_of: Option<Box2>,
    children: Vec<Node>,
    /// This node has no children AND sits at the capture depth, so whether it
    /// has any is unknown.
    truncated: bool,
}

#[derive(Debug, Clone, Copy)]
struct Box2 {
    x: f64,
    width: f64,
    height: f64,
}

/// The authority class a layer name declares, or `None`.
///
/// THE ASYMMETRY IS THE WHOLE FINDING and it is not a compromise. Splitting
/// every authority name and every quarantined sibling in the subject on the
/// separator yielded twelve distinct leading segments that divide cleanly, and
/// the two halves have to be matched differently:
///
/// - a QUARANTINE marker is matched against the LEADING SEGMENT only. Matching
///   anywhere excluded nine current screens whose names merely MENTION a target
///   (`Screen - /matters/[id] - Profile - source contract + target recovery`),
///   two of them the busiest surfaces in the product, on a word in a sentence.
/// - an AUTHORITY marker is matched ANYWHERE and case-insensitively, because
///   `/dashboards - current source matter master-detail` puts its marker
///   mid-name and in lower case, and anchoring it resolved the founding route
///   of the whole workstream to a layer whose body is switched off.
pub fn authority_of(name: &str, config: &ScreensConfig) -> Option<Authority> {
    let lowered = name.to_lowercase();
    let head = match lowered.split_once(config.name_separator.as_str()) {
        Some((first, _)) => first.trim(),
        None => lowered.trim(),
    };
    for marker in &config.quarantine_markers {
        if head.contains(&marker.to_lowercase()) {
            return Some(Authority::Quarantined);
        }
    }
    for marker in &config.authority_markers {
        if lowered.contains(&marker.to_lowercase()) {
            return Some(Authority::Current);
        }
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Authority {
    Current,
    Quarantined,
}

/// Build the ledger from one or more saved `nodes` responses.
///
/// Several bodies rather than one, because the `ids` list goes in the query
/// string and a real capture is batched. The batches are merged by node id, so
/// re-capturing a batch replaces its rows rather than duplicating them.
pub fn build_ledger(
    file_key: &str,
    bodies: &[String],
    config: &ScreensConfig,
    source_description: &str,
) -> Result<FrameLedger> {
    if bodies.is_empty() {
        return Err(VdsError::precondition(
            "no capture was given, so there is nothing to derive a frame ledger from. A ledger \
             built from zero batches would claim the file draws no screens.\n  \
             Capture the frames out of band and pass them with --from.",
        ));
    }

    // Raw first, so the capture depth can be derived across ALL batches before
    // any node is marked. Deriving per batch would let a shallow batch mark
    // leaves a deeper batch proves are real.
    let mut raw: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    for body in bodies {
        let payload: serde_json::Value = serde_json::from_str(body).map_err(|e| {
            VdsError::precondition(format!(
                "a capture is not JSON: {e}. A partial parse would produce a ledger claiming \
                 fewer frames than were captured, and every proof reading it would be narrower \
                 than it looks."
            ))
        })?;
        let nodes = payload
            .get("nodes")
            .and_then(|v| v.as_object())
            .ok_or_else(|| {
                VdsError::precondition(
                    "a capture carries no `nodes` object. This reads a saved \
                 `GET /v1/files/:key/nodes` response; a `GET /v1/files/:key` response has a \
                 `document` instead and holds the whole file, which is a different shape and a \
                 different question.",
                )
            })?;
        for (node_id, wrapper) in nodes {
            if let Some(document) = wrapper.get("document") {
                raw.insert(normalise_node_id(node_id), document.clone());
            }
        }
    }

    if raw.is_empty() {
        return Err(VdsError::precondition(
            "the capture resolved no nodes. Every entry carried no `document`, which is what \
             Figma returns for a node id that does not exist in this file. A ledger built from \
             it would say the file draws no screens, which is a claim about the file rather \
             than about the capture.",
        ));
    }

    let capture_depth = raw.values().map(|n| depth_of(n, 0)).max().unwrap_or(0);

    let mut frames = Vec::new();
    let mut truncated_leaves = 0u32;
    for (node_id, document) in &raw {
        let node = read_node(document, capture_depth, 0, &mut truncated_leaves);
        frames.push(row_for(node_id, &node, config));
    }
    frames.sort_by(|a, b| a.node_id.cmp(&b.node_id));

    let disclaimed = frames.iter().filter(|f| f.disclaimed).count();
    let truncated = frames.iter().filter(|f| f.truncated).count();
    let unlabelled = frames
        .iter()
        .filter(|f| f.authority_by == AuthorityBy::Unlabelled)
        .count();

    let mut notes = vec![
        format!("derived from {source_description}"),
        "This ledger records frame names, layer names, region names and a COUNT of content \
         columns. It records no width, height, colour, font or radius: those stay in the Figma \
         file, which [2026] VJS-CC-OPBOX 3 D1 makes the system of record for what is decided, \
         and a length under `.vds/**` is the storing form VDS S-2(2) prohibits."
            .to_owned(),
        format!(
            "[capture-depth] the deepest chain in this capture is {capture_depth}, and \
             {truncated_leaves} leaf node(s) sit at it. A Figma response carries no \"this \
             subtree was cut off\" flag, so a childless node at that depth and a genuinely \
             empty one are the same bytes. Every frame whose column derivation read one is \
             marked `truncated`, and no proof may enforce such a row: \"the frame draws \
             nothing here\" and \"we did not look\" must not be the same value."
        ),
    ];
    if truncated > 0 {
        notes.push(format!(
            "{truncated} frame(s) derived their column count from a subtree that reaches the \
             capture boundary. Re-capture deeper before relying on those rows."
        ));
    }
    if disclaimed > 0 {
        notes.push(format!(
            "{disclaimed} frame(s) DISCLAIM THEMSELVES: their authoritative layer says in its \
             own name that it is not source-current, or was never built. Such a frame states no \
             contract, so measuring anything against it produces a difference that is real and \
             means nothing."
        ));
    }
    if unlabelled > 0 {
        notes.push(format!(
            "{unlabelled} frame(s) carry no authority marker at all, so the frame itself was \
             taken as the authority. That is a default and not a declaration: nothing in those \
             frames SAID it was current, and if this file uses different words for that, they \
             belong in `[screens] authority_markers`."
        ));
    }

    let mut ledger = FrameLedger {
        schema_version: LEDGER_SCHEMA_VERSION,
        generated_at: Timestamp::now(),
        generated_by: GENERATOR_COMMAND.to_owned(),
        file_key: file_key.to_owned(),
        capture_depth,
        truncated_leaves,
        content_digest: Digest::of_text("placeholder"),
        frames,
        notes,
    };
    ledger.content_digest = ledger.compute_content_digest()?;
    Ok(ledger)
}

/// The deepest chain under a node, in edges.
fn depth_of(node: &serde_json::Value, at: u32) -> u32 {
    match node.get("children").and_then(|v| v.as_array()) {
        Some(children) if !children.is_empty() => children
            .iter()
            .map(|child| depth_of(child, at + 1))
            .max()
            .unwrap_or(at),
        _ => at,
    }
}

/// Read one node, marking every childless leaf that sits at the capture depth.
fn read_node(
    value: &serde_json::Value,
    capture_depth: u32,
    at: u32,
    truncated_leaves: &mut u32,
) -> Node {
    let children: Vec<Node> = value
        .get("children")
        .and_then(|v| v.as_array())
        .map(|array| {
            array
                .iter()
                .map(|child| read_node(child, capture_depth, at + 1, truncated_leaves))
                .collect()
        })
        .unwrap_or_default();

    let truncated = children.is_empty() && at >= capture_depth;
    if truncated {
        *truncated_leaves += 1;
    }

    Node {
        name: value
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_owned(),
        // Figma omits `visible` when it is true. An INVISIBLE layer is a
        // drawing the designer switched off, and counting one is counting
        // something nobody can see in Figma.
        visible: value
            .get("visible")
            .and_then(|v| v.as_bool())
            .unwrap_or(true),
        box_of: box_of(value),
        children,
        truncated,
    }
}

fn box_of(value: &serde_json::Value) -> Option<Box2> {
    let b = value.get("absoluteBoundingBox")?;
    Some(Box2 {
        x: b.get("x")?.as_f64()?,
        width: b.get("width")?.as_f64()?,
        height: b.get("height")?.as_f64()?,
    })
}

/// The subtree that GOVERNS this frame, and how that was decided.
fn authority_root<'a>(frame: &'a Node, config: &ScreensConfig) -> (&'a Node, AuthorityBy) {
    let labelled: Vec<&Node> = frame
        .children
        .iter()
        .filter(|c| authority_of(&c.name, config) == Some(Authority::Current))
        .collect();
    if !labelled.is_empty() {
        // Prefer a VISIBLE one. An invisible current layer is a draft the
        // designer switched off, and building from it produces a screen nobody
        // can see in Figma.
        let chosen = labelled
            .iter()
            .find(|c| c.visible)
            .copied()
            .unwrap_or(labelled[0]);
        return (chosen, AuthorityBy::NamedLayer);
    }
    if frame
        .children
        .iter()
        .any(|c| config.region_names.iter().any(|r| r == &c.name))
    {
        return (frame, AuthorityBy::FrameOwnChildren);
    }
    (frame, AuthorityBy::Unlabelled)
}

fn row_for(node_id: &str, frame: &Node, config: &ScreensConfig) -> FrameRow {
    let (root, authority_by) = authority_root(frame, config);

    let quarantined: Vec<String> = frame
        .children
        .iter()
        .filter(|c| authority_of(&c.name, config) == Some(Authority::Quarantined))
        .map(|c| c.name.clone())
        .collect();

    // A FRAME MAY DISCLAIM ITSELF, and in the subject 25 of 188 did. The test
    // is on the AUTHORITY's name, not the frame's, because the authority is what
    // any comparison would be made against.
    let disclaimed = authority_of(&root.name, config) == Some(Authority::Quarantined);

    let visible_children: Vec<&Node> = root.children.iter().filter(|c| c.visible).collect();
    let mut regions: Vec<String> = Vec::new();
    for name in &config.region_names {
        if visible_children.iter().any(|c| &c.name == name) {
            regions.push(name.clone());
        }
    }

    // Shape three: the authoritative layer IS the body content, with the shell
    // alongside it. Where a named body region exists the columns live inside it;
    // otherwise the authority's own children are the bands.
    let container = visible_children
        .iter()
        .find(|c| regions.iter().any(|r| r == &c.name) && is_body(&c.name))
        .copied()
        .unwrap_or(root);

    let (columns, truncated) = content_columns(container);
    FrameRow {
        node_id: node_id.to_owned(),
        frame_name: frame.name.clone(),
        authority_layer: root.name.clone(),
        authority_by,
        quarantined,
        disclaimed,
        regions,
        columns,
        truncated,
    }
}

/// Whether a region name is the one that holds the content.
///
/// The body is the region the columns live in; every other region is chrome.
/// This is the one place VDS guesses at the subject's vocabulary, and it is
/// bounded: where a project names its content region something else, the
/// derivation falls back to the authority's own children and still finds the
/// columns. The guess can therefore cost precision and never correctness.
fn is_body(name: &str) -> bool {
    name.eq_ignore_ascii_case("body") || name.eq_ignore_ascii_case("content")
}

/// How many side-by-side content panes this container draws, and whether the
/// derivation had to read a node whose children it could not see.
///
/// CLUSTERED BY X, NOT PAIRED AS SIBLINGS, and the difference is a whole shape
/// of screen. Requiring the children themselves to be disjoint left-to-right
/// only sees a layout drawn as one frame per column; a route in the subject is
/// six surfaces, four stacked in one x-interval and two in another, which is
/// plainly two columns and which the sibling test reported as none.
///
/// Returns 1 rather than 0 where nothing clusters. A screen with no split still
/// has one content pane, and reporting 0 made the prior art score a route
/// `frame=0 code=1` and call agreement a deviation. One register for both sides.
///
/// # What counts as truncated, and why it is not "any leaf at the boundary"
///
/// EVERY capture ends somewhere, so in a real one the deepest panes are always
/// childless-at-the-boundary. Marking a frame truncated for those would mark
/// every frame, and a gate that excludes everything is a gate that measures
/// nothing while printing a reason. The reading is unreliable in exactly one
/// case: when the node whose CHILDREN this derivation clustered is itself a
/// boundary leaf, so its child list is empty because nothing fetched it. That
/// is precisely the prior art's defect: a content frame reported `children: []`
/// and the ledger wrote "draws nothing" as a fact, and it has four children.
fn content_columns(container: &Node) -> (u32, bool) {
    // The container's own children are unknown, so an empty band list here is
    // "we did not look" and not "the frame draws nothing".
    if container.truncated {
        return (1, true);
    }

    let bands: Vec<&Node> = container.children.iter().filter(|c| c.visible).collect();
    if let Some(count) = cluster(&bands) {
        return (count, false);
    }

    // Otherwise the columns live inside the tallest band: a header band above a
    // content band is the common shape, and the columns are the content band's
    // children.
    let mut tall: Vec<&&Node> = bands
        .iter()
        .filter(|b| {
            b.box_of
                .map(|x| x.height >= MIN_CONTENT_BAND_HEIGHT)
                .unwrap_or(false)
        })
        .collect();
    tall.sort_by(|a, b| {
        b.box_of
            .map(|x| x.height)
            .unwrap_or(0.0)
            .total_cmp(&a.box_of.map(|x| x.height).unwrap_or(0.0))
    });
    let mut reached_boundary = false;
    for band in tall {
        if band.truncated {
            // This band could hold the columns and nobody fetched its children.
            reached_boundary = true;
            continue;
        }
        let children: Vec<&Node> = band.children.iter().filter(|c| c.visible).collect();
        if let Some(count) = cluster(&children) {
            return (count, false);
        }
    }
    (1, reached_boundary)
}

/// The number of x-disjoint clusters among these nodes, or `None` where there
/// is no side-by-side arrangement to report.
fn cluster(nodes: &[&Node]) -> Option<u32> {
    let boxed: Vec<(&Node, Box2)> = nodes
        .iter()
        .filter_map(|n| n.box_of.map(|b| (*n, b)))
        .filter(|(_, b)| b.width >= MIN_COLUMN_WIDTH && b.height >= MIN_COLUMN_HEIGHT)
        .collect();
    if boxed.len() < 2 {
        return None;
    }
    let left = boxed.iter().map(|(_, b)| b.x).fold(f64::INFINITY, f64::min);
    let right = boxed
        .iter()
        .map(|(_, b)| b.x + b.width)
        .fold(f64::NEG_INFINITY, f64::max);
    let span = right - left;
    if span <= 0.0 {
        return None;
    }

    // A band spanning nearly the whole width is a HEADER or a rule, not a
    // column, and including it merges every column into one because it overlaps
    // them all.
    let mut columns: Vec<Box2> = boxed
        .iter()
        .map(|(_, b)| *b)
        .filter(|b| b.width < FULL_WIDTH_FRACTION * span)
        .collect();
    if columns.len() < 2 {
        return None;
    }
    columns.sort_by(|a, b| a.x.total_cmp(&b.x));

    let mut groups: Vec<(f64, f64)> = Vec::new();
    for b in &columns {
        let (x0, x1) = (b.x, b.x + b.width);
        let mut merged = false;
        for group in groups.iter_mut() {
            if x0 < group.1 - SEAM_SLACK && x1 > group.0 + SEAM_SLACK {
                group.0 = group.0.min(x0);
                group.1 = group.1.max(x1);
                merged = true;
                break;
            }
        }
        if !merged {
            groups.push((x0, x1));
        }
    }
    (groups.len() >= 2).then_some(groups.len() as u32)
}

/// Figma writes a node id as `12:34` in a file URL and `12-34` in a deep link.
/// They are the same node, and a ledger that treated them as different would
/// report a frame unresolved because the designer copied the other spelling.
pub fn normalise_node_id(raw: &str) -> String {
    raw.replace('-', ":")
}

/// Every node id in the ledger, for reporting which of them nobody claims.
pub fn node_ids(ledger: &FrameLedger) -> BTreeSet<String> {
    ledger.frames.iter().map(|f| f.node_id.clone()).collect()
}

// ------------------------------------------------------------------------ io

/// Where the frame ledger lives, per `[screens] frames_ledger`.
pub fn ledger_path(project: &vds_core::Project) -> std::path::PathBuf {
    project.root.join(&project.config.screens.frames_ledger)
}

pub fn write(project: &vds_core::Project, ledger: &FrameLedger) -> Result<std::path::PathBuf> {
    let path = ledger_path(project);
    let text = serde_yaml::to_string(ledger).map_err(|e| VdsError::Serialize {
        what: "the frame ledger".into(),
        message: e.to_string(),
    })?;
    vds_core::write_text_atomically(&path, &text)?;
    Ok(path)
}

pub fn read(project: &vds_core::Project) -> Result<Option<FrameLedger>> {
    let path = ledger_path(project);
    if !path.is_file() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path).map_err(|e| VdsError::io(path.display(), e))?;
    // The version is read from the RAW value before the typed parse, so a
    // ledger from a future build is refused rather than half-understood
    // (VDS S-11(2)). A loader that skipped the fields it could not parse would
    // compare a ledger it only half read and call the difference a deviation.
    let raw: serde_yaml::Value = serde_yaml::from_str(&text).map_err(|e| VdsError::Artefact {
        path: project.rel(&path),
        message: format!("is not readable YAML: {e}"),
    })?;
    let found = raw
        .get("schema_version")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    if found > LEDGER_SCHEMA_VERSION {
        return Err(VdsError::SchemaVersionAhead {
            path: project.rel(&path),
            kind: "frame ledger",
            found,
            understood: LEDGER_SCHEMA_VERSION,
        });
    }
    let ledger: FrameLedger = serde_yaml::from_value(raw).map_err(|e| VdsError::Artefact {
        path: project.rel(&path),
        message: format!("is not a frame ledger: {e}"),
    })?;
    Ok(Some(ledger))
}

/// The decided-target file every SCREEN record agrees on, or an explanation.
///
/// The screen register's own answer to the question
/// [`crate::pull::declared_file_key`] asks of the component register. Two files
/// is two opinions about what is decided.
pub fn declared_file_key(store: &vds_store::Store) -> Result<Option<String>> {
    let records = store.read_screens()?;
    let mut keys: Vec<String> = records
        .iter()
        .filter_map(|r| r.value.frame.as_ref().map(|f| f.file_key.clone()))
        .collect();
    keys.sort();
    keys.dedup();
    match keys.len() {
        0 => Ok(None),
        1 => Ok(Some(keys.remove(0))),
        _ => Err(VdsError::precondition(format!(
            "the screen register names {} different Figma files: {}.\n  \
             [2026] VJS-CC-OPBOX 3 D1 names ONE decided-target file as the system of record \
             for what is decided, and two of them is two opinions about what is decided. Pass \
             --file-key to say which, or amend the records that disagree.",
            keys.len(),
            keys.join(", ")
        ))),
    }
}

/// Read saved captures from disk and build a ledger from them.
///
/// The reproducible path, and the only one: there is deliberately no API
/// transport here. `vds figma pull` has one because the `files` endpoint is a
/// single request; a screen file's frames come in batches with an ids list in
/// the query string, and building that batching into VDS would put a retry
/// loop, a resume and a rate limit inside the tool. The capture is the
/// subject's to run, out of band, with its own token (see
/// `crates/vds-figma/src/pull.rs:79` for why a token never enters `.vds/`).
pub fn from_saved(
    file_key: &str,
    paths: &[std::path::PathBuf],
    config: &ScreensConfig,
) -> Result<FrameLedger> {
    let mut bodies = Vec::new();
    for path in paths {
        bodies.push(std::fs::read_to_string(path).map_err(|e| VdsError::io(path.display(), e))?);
    }
    let description = match paths.len() {
        1 => format!("a saved nodes response at {}", paths[0].display()),
        n => format!(
            "{n} saved nodes responses, the first at {}",
            paths[0].display()
        ),
    };
    build_ledger(file_key, &bodies, config, &description)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> ScreensConfig {
        ScreensConfig::default()
    }

    /// `x`, `width`, `height`, and children.
    fn band(
        name: &str,
        x: f64,
        width: f64,
        height: f64,
        children: serde_json::Value,
    ) -> serde_json::Value {
        serde_json::json!({
            "id": "1:1",
            "name": name,
            "type": "FRAME",
            "absoluteBoundingBox": {"x": x, "y": 0, "width": width, "height": height},
            "children": children,
        })
    }

    fn capture(nodes: serde_json::Value) -> String {
        serde_json::json!({"nodes": nodes}).to_string()
    }

    // -- the authority asymmetry, both directions ----------------------------

    /// The half that was learned the hard way. `/dashboards` puts its marker
    /// MID-NAME and in LOWER CASE, and a prefix match resolved the founding
    /// route of the whole workstream to a layer whose body is switched off.
    #[test]
    fn an_authority_marker_is_matched_anywhere_and_case_insensitively() {
        assert_eq!(
            authority_of(
                "/dashboards · current source matter master-detail",
                &config()
            ),
            Some(Authority::Current)
        );
        assert_eq!(
            authority_of("CURRENT SOURCE · /finance/invoices", &config()),
            Some(Authority::Current)
        );
        assert_eq!(authority_of("Screen · /settings", &config()), None);
    }

    /// The other half, and the one that cost nine false exclusions. A hybrid
    /// name that MENTIONS a target is a current screen; matching quarantine
    /// anywhere took two of the busiest surfaces in the product out of the
    /// contract on a word in a sentence.
    #[test]
    fn a_quarantine_marker_only_counts_as_the_leading_segment() {
        assert_eq!(
            authority_of("LEGACY UNDERLAY · body", &config()),
            Some(Authority::Quarantined)
        );
        assert_eq!(
            authority_of("TARGET · /oversight/agents [no page.tsx]", &config()),
            Some(Authority::Quarantined)
        );
        assert_eq!(
            authority_of(
                "Screen · /matters/[id] · Profile · source contract + target recovery",
                &config()
            ),
            None,
            "a current screen that merely mentions a target is not quarantined"
        );
    }

    /// A frame carrying both must resolve to the current one. This is the
    /// precedence the marker order encodes, and getting it wrong builds the
    /// retired screen and reports success.
    #[test]
    fn a_current_layer_wins_over_a_legacy_underlay_beside_it() {
        let body = |n: usize| {
            let kids: Vec<serde_json::Value> = (0..n)
                .map(|i| {
                    band(
                        "child",
                        i as f64 * 400.0,
                        380.0,
                        700.0,
                        serde_json::json!([]),
                    )
                })
                .collect();
            serde_json::json!(kids)
        };
        let frame = band(
            "Screen · /matters",
            0.0,
            1440.0,
            900.0,
            serde_json::json!([
                band("LEGACY UNDERLAY · body", 0.0, 1440.0, 900.0, body(4)),
                band(
                    "CURRENT SOURCE CONTRACT · /matters",
                    0.0,
                    1440.0,
                    900.0,
                    body(2)
                ),
            ]),
        );
        let ledger = build_ledger(
            "KEY",
            &[capture(serde_json::json!({"669:2": {"document": frame}}))],
            &config(),
            "a test",
        )
        .unwrap();
        let row = ledger.row("669:2").unwrap();
        assert_eq!(row.authority_layer, "CURRENT SOURCE CONTRACT · /matters");
        assert_eq!(row.authority_by, AuthorityBy::NamedLayer);
        assert_eq!(
            row.quarantined,
            vec!["LEGACY UNDERLAY · body".to_string()],
            "the superseded layer is RECORDED, because \"this route has a legacy underlay\" is \
             exactly the fact a reader needs in order to not build from it"
        );
        assert_eq!(
            row.columns, 2,
            "the CURRENT layer draws two, the legacy four"
        );
    }

    #[test]
    fn an_invisible_current_layer_loses_to_a_visible_one() {
        let mut hidden = band(
            "CURRENT SOURCE · draft",
            0.0,
            1440.0,
            900.0,
            serde_json::json!([]),
        );
        hidden["visible"] = serde_json::json!(false);
        let frame = band(
            "Screen · /x",
            0.0,
            1440.0,
            900.0,
            serde_json::json!([
                hidden,
                band(
                    "CURRENT SOURCE · shipped",
                    0.0,
                    1440.0,
                    900.0,
                    serde_json::json!([])
                ),
            ]),
        );
        let ledger = build_ledger(
            "KEY",
            &[capture(serde_json::json!({"1:2": {"document": frame}}))],
            &config(),
            "a test",
        )
        .unwrap();
        assert_eq!(
            ledger.row("1:2").unwrap().authority_layer,
            "CURRENT SOURCE · shipped",
            "an invisible current layer is a draft the designer switched off"
        );
    }

    // -- self-disclaiming frames ---------------------------------------------

    #[test]
    fn a_frame_that_disclaims_itself_is_recorded_as_stating_no_contract() {
        let frame = band(
            "LEGACY / TARGET REFERENCE · /sql · NOT SOURCE CURRENT",
            0.0,
            1440.0,
            900.0,
            serde_json::json!([]),
        );
        let ledger = build_ledger(
            "KEY",
            &[capture(serde_json::json!({"3:3": {"document": frame}}))],
            &config(),
            "a test",
        )
        .unwrap();
        assert!(ledger.row("3:3").unwrap().disclaimed);
        assert!(
            ledger
                .notes
                .iter()
                .any(|n| n.contains("DISCLAIM THEMSELVES")),
            "{:?}",
            ledger.notes
        );
    }

    // -- the depth finding ---------------------------------------------------

    /// AN UNSEEN CHILD IS NOT AN ABSENT CHILD. The payload for a truncated
    /// subtree and for a genuinely empty one are the same bytes; only the depth
    /// asked for knows the difference.
    ///
    /// The frame here is the prior art's own case: a content band sitting
    /// EXACTLY on the capture boundary, whose child list is empty because
    /// nothing fetched it. The ledger recorded that as "draws nothing" about
    /// one of the two busiest routes in the product, and it has four children.
    #[test]
    fn a_container_at_the_capture_boundary_is_marked_rather_than_read_as_empty() {
        let frame = band(
            "Screen · /matters",
            0.0,
            1440.0,
            900.0,
            serde_json::json!([band(
                "body",
                0.0,
                1440.0,
                900.0,
                serde_json::json!([band("content", 0.0, 1440.0, 800.0, serde_json::json!([]))])
            )]),
        );
        let ledger = build_ledger(
            "KEY",
            &[capture(serde_json::json!({"1:1": {"document": frame}}))],
            &config(),
            "a test",
        )
        .unwrap();
        assert_eq!(ledger.capture_depth, 2);
        assert!(
            ledger.row("1:1").unwrap().truncated,
            "the content band is childless AT the boundary, so whether it draws columns is \
             unknown and the count of 1 states an absence nothing observed"
        );
        assert!(ledger.truncated_leaves > 0);
        assert!(
            ledger.notes.iter().any(|n| n.contains("we did not look")),
            "{:?}",
            ledger.notes
        );
    }

    /// The other direction, and the one that keeps the guard from swallowing
    /// the gate. EVERY capture ends somewhere, so in a real one the deepest
    /// panes are always childless at the boundary. Marking a frame truncated
    /// for those marks every frame, and a gate that excludes everything
    /// measures nothing while printing a reason for it.
    #[test]
    fn ordinary_leaf_panes_at_the_boundary_do_not_make_a_reading_truncated() {
        let frame = band(
            "Screen · /orders",
            0.0,
            1440.0,
            900.0,
            serde_json::json!([band(
                "body",
                0.0,
                1440.0,
                900.0,
                serde_json::json!([
                    band("list", 0.0, 900.0, 700.0, serde_json::json!([])),
                    band("detail", 920.0, 500.0, 700.0, serde_json::json!([])),
                ])
            )]),
        );
        let ledger = build_ledger(
            "KEY",
            &[capture(serde_json::json!({"1:1": {"document": frame}}))],
            &config(),
            "a test",
        )
        .unwrap();
        let row = ledger.row("1:1").unwrap();
        assert!(
            ledger.truncated_leaves > 0,
            "the panes ARE at the boundary, and the ledger says so"
        );
        assert!(
            !row.truncated,
            "but the derivation clustered the body's children, and the body was fully read. \
             Their own children could not have changed the count."
        );
        assert_eq!(row.columns, 2);
    }

    // -- the column derivation -----------------------------------------------

    /// The single most important threshold. A page header above two panes is
    /// the ordinary shape of a screen, and counting the header as a column
    /// merges everything into one because it overlaps them all.
    #[test]
    fn a_full_width_header_is_not_a_column() {
        let frame = band(
            "Screen · /x",
            0.0,
            1440.0,
            900.0,
            serde_json::json!([band(
                "body",
                0.0,
                1440.0,
                900.0,
                serde_json::json!([
                    band("PageHeader", 0.0, 1440.0, 96.0, serde_json::json!([])),
                    band("list", 0.0, 900.0, 700.0, serde_json::json!([])),
                    band("detail", 920.0, 500.0, 700.0, serde_json::json!([])),
                ])
            )]),
        );
        let ledger = build_ledger(
            "KEY",
            &[capture(serde_json::json!({"1:1": {"document": frame}}))],
            &config(),
            "a test",
        )
        .unwrap();
        assert_eq!(
            ledger.row("1:1").unwrap().columns,
            2,
            "counting the header as a column reports ONE, every time, on the majority of screens"
        );
    }

    /// CLUSTERED BY X, NOT PAIRED AS SIBLINGS. Four surfaces stacked in one
    /// x-interval and two in another is two columns; the sibling test reports
    /// none, and the route is then scored a one-pane screen whose code renders
    /// three, which is a defect that does not exist.
    #[test]
    fn stacked_surfaces_in_two_x_intervals_are_two_columns() {
        let mut kids = Vec::new();
        for _ in 0..4 {
            kids.push(band("surface", 1580.0, 812.0, 300.0, serde_json::json!([])));
        }
        for _ in 0..2 {
            kids.push(band("surface", 2416.0, 460.0, 300.0, serde_json::json!([])));
        }
        let frame = band(
            "Screen · /finance/invoices/[id]",
            1580.0,
            1296.0,
            900.0,
            serde_json::json!([band("body", 1580.0, 1296.0, 900.0, serde_json::json!(kids))]),
        );
        let ledger = build_ledger(
            "KEY",
            &[capture(serde_json::json!({"1:1": {"document": frame}}))],
            &config(),
            "a test",
        )
        .unwrap();
        assert_eq!(ledger.row("1:1").unwrap().columns, 2);
    }

    /// ONE REGISTER FOR BOTH SIDES. A screen with no split still has one
    /// content pane, and writing that as 0 made the prior art score a route
    /// `frame=0 code=1` and call agreement a deviation.
    #[test]
    fn a_single_pane_screen_draws_one_column_and_never_zero() {
        let frame = band(
            "Screen · /x",
            0.0,
            1440.0,
            900.0,
            serde_json::json!([band(
                "body",
                0.0,
                1440.0,
                900.0,
                serde_json::json!([band("only", 0.0, 1440.0, 700.0, serde_json::json!([]))])
            )]),
        );
        let ledger = build_ledger(
            "KEY",
            &[capture(serde_json::json!({"1:1": {"document": frame}}))],
            &config(),
            "a test",
        )
        .unwrap();
        assert_eq!(ledger.row("1:1").unwrap().columns, 1);
    }

    #[test]
    fn the_regions_found_are_reported_in_the_configured_order() {
        let frame = band(
            "Screen · /x",
            0.0,
            1440.0,
            900.0,
            serde_json::json!([
                band("body", 224.0, 1216.0, 860.0, serde_json::json!([])),
                band("rail", 0.0, 224.0, 860.0, serde_json::json!([])),
            ]),
        );
        let ledger = build_ledger(
            "KEY",
            &[capture(serde_json::json!({"1:1": {"document": frame}}))],
            &config(),
            "a test",
        )
        .unwrap();
        let row = ledger.row("1:1").unwrap();
        assert_eq!(row.regions, vec!["rail".to_string(), "body".to_string()]);
        assert_eq!(row.authority_by, AuthorityBy::FrameOwnChildren);
    }

    // -- the ledger contract -------------------------------------------------

    #[test]
    fn the_two_node_id_spellings_are_the_same_frame() {
        let frame = band("Screen", 0.0, 100.0, 100.0, serde_json::json!([]));
        let ledger = build_ledger(
            "KEY",
            &[capture(serde_json::json!({"669-2": {"document": frame}}))],
            &config(),
            "a test",
        )
        .unwrap();
        assert!(
            ledger.row("669:2").is_some(),
            "12-34 from a deep link and 12:34 from a file URL are the same node"
        );
    }

    #[test]
    fn two_builds_of_one_capture_produce_one_content_digest() {
        let frame = band("Screen", 0.0, 100.0, 100.0, serde_json::json!([]));
        let body = capture(serde_json::json!({"1:1": {"document": frame}}));
        let a = build_ledger("KEY", std::slice::from_ref(&body), &config(), "a test").unwrap();
        let b = build_ledger("KEY", &[body], &config(), "a test").unwrap();
        assert_eq!(a.content_digest, b.content_digest);
    }

    #[test]
    fn the_content_digest_excludes_generated_at() {
        let frame = band("Screen", 0.0, 100.0, 100.0, serde_json::json!([]));
        let mut ledger = build_ledger(
            "KEY",
            &[capture(serde_json::json!({"1:1": {"document": frame}}))],
            &config(),
            "a test",
        )
        .unwrap();
        let before = ledger.compute_content_digest().unwrap();
        ledger.generated_at = Timestamp::fixed(2000, 1, 1, 0, 0, 0);
        assert_eq!(before, ledger.compute_content_digest().unwrap());
    }

    #[test]
    fn a_hand_edited_ledger_is_refused() {
        let frame = band("Screen", 0.0, 100.0, 100.0, serde_json::json!([]));
        let mut ledger = build_ledger(
            "KEY",
            &[capture(serde_json::json!({"1:1": {"document": frame}}))],
            &config(),
            "a test",
        )
        .unwrap();
        check_fresh(&ledger, Some("KEY")).unwrap();
        ledger.frames[0].columns = 9;
        let error = check_fresh(&ledger, None).unwrap_err();
        assert!(error.to_string().contains("was edited"), "{error}");
    }

    #[test]
    fn a_ledger_from_another_file_is_refused() {
        let frame = band("Screen", 0.0, 100.0, 100.0, serde_json::json!([]));
        let ledger = build_ledger(
            "KEY",
            &[capture(serde_json::json!({"1:1": {"document": frame}}))],
            &config(),
            "a test",
        )
        .unwrap();
        let error = check_fresh(&ledger, Some("OTHER")).unwrap_err();
        assert!(error.to_string().contains("two opinions"), "{error}");
    }

    #[test]
    fn a_capture_of_the_wrong_endpoint_is_refused_rather_than_read_as_empty() {
        let error = build_ledger(
            "KEY",
            &[serde_json::json!({"document": {"id": "0:0"}}).to_string()],
            &config(),
            "a test",
        )
        .unwrap_err();
        assert!(error.to_string().contains("`nodes` object"), "{error}");
    }

    #[test]
    fn a_capture_of_nothing_is_refused_rather_than_producing_an_empty_ledger() {
        let error = build_ledger("KEY", &[], &config(), "a test").unwrap_err();
        assert!(error.to_string().contains("draws no screens"), "{error}");

        let error = build_ledger(
            "KEY",
            &[capture(serde_json::json!({"9:9": {"err": "not found"}}))],
            &config(),
            "a test",
        )
        .unwrap_err();
        assert!(error.to_string().contains("resolved no nodes"), "{error}");
    }

    /// VDS S-2(2). There is no field here a width could live in, and this test
    /// holds the serialised form to that. The prior art records
    /// `columns: [924, 420]`, which under `.vds/**` is the storing form: a
    /// reader recovers two design values by reading it, and `no_stored_values`
    /// would then fail forever on a file VDS wrote itself.
    ///
    /// The NOTES are cleared first, and that is not the test being softened.
    /// `crates/vds-core/src/lib.rs:110` settles the same question the same way
    /// for the artefact schemas: property NAMES only, because a description may
    /// legitimately use the word "width" while explaining that there is no
    /// width field. What a note may never carry is a VALUE, and the second half
    /// below holds it to that.
    #[test]
    fn a_serialised_frame_ledger_names_no_realisation() {
        let frame = band(
            "Screen · /x",
            0.0,
            1440.0,
            900.0,
            serde_json::json!([band(
                "body",
                0.0,
                1440.0,
                900.0,
                serde_json::json!([
                    band("list", 0.0, 900.0, 700.0, serde_json::json!([])),
                    band("detail", 920.0, 500.0, 700.0, serde_json::json!([])),
                ])
            )]),
        );
        let ledger = build_ledger(
            "KEY",
            &[capture(serde_json::json!({"1:1": {"document": frame}}))],
            &config(),
            "a test",
        )
        .unwrap();

        let structure = FrameLedger {
            notes: vec![],
            ..ledger.clone()
        };
        let text = serde_yaml::to_string(&structure).unwrap();
        for forbidden in [
            "width",
            "height",
            "px",
            "rem",
            "colour",
            "color",
            "fill",
            "stroke",
            "cornerRadius",
            "opacity",
            "#",
        ] {
            assert!(
                !text.contains(forbidden),
                "the frame ledger declares {forbidden:?}, which is a realisation and belongs in \
                 the Figma file, not in .vds/ (VDS S-2(2)): {text}"
            );
        }

        // The measurements themselves, in every form. These are the numbers the
        // prior art wrote down, and none of them may survive the derivation.
        let whole = serde_yaml::to_string(&ledger).unwrap();
        for value in ["1440", "900", "920", "700", "500"] {
            assert!(
                !whole.contains(value),
                "the frame ledger carries {value:?}, a measurement taken off the drawing. A \
                 COUNT of columns is a requirement's shape (VDS S-2(6)); a width is the design's \
                 own answer and stays in Figma: {whole}"
            );
        }
    }
}
