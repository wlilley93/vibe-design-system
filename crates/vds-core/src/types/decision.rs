//! The Principal's per-frame decisions, and the door that admits one.
//!
//! [2026] VJS-FI-VDS 2 order 4. CC-OPBOX 6 makes registration conditional on
//! **(a)** a recognised authority label **OR** **(b)** an express Principal
//! act, and the register implemented limb (a) alone. That is not a partial
//! enforcement, it is an INVERSION, because limb (a) is satisfied by precisely
//! the artefacts limb (b) exists to displace: where the labels were
//! machine-implanted, a door that admits only labelled subjects admits the
//! implant and refuses every subject the Principal cleaned. In the subject
//! estate it refused 118 of 127 signed frames outright and admitted the other 9
//! on the strength of the layer the Principal had expressly overridden.
//!
//! This module is limb (b). It carries no authority of its own: it reads a
//! decision the Principal took elsewhere, checks that the decision is about
//! THIS frame and says exactly `sign`, and refuses everything else. Recording
//! is not granting.
//!
//! # Why the evidence must be per-frame and can never be an aggregate
//!
//! An attestation over the frames ledger AS A WHOLE says nothing about any one
//! frame in it. The ledger is the record that CLASSIFIES the frames, so a
//! signature over the ledger's own digest attests to the classification and not
//! to any subject of it - it is not a label-resolution act (CC-OPBOX 7 R5, and
//! forbidden expressly by [2026] VJS-FI-VDS 2). [`PrincipalDecision::parse`]
//! therefore refuses an aggregate-scoped document BY NAME rather than letting
//! it fail as a malformed decision, because a refusal that says "this does not
//! parse" invites a reader to fix the shape and try again.
//!
//! # Why the door digests the decision's own bytes
//!
//! A decision file a caller can edit is a decision a caller can author. The
//! O2 export records a digest for every row it exports, and the door checks the
//! bytes it was handed against that record. A decision that is not the exported
//! decision is somebody's opinion about what the Principal decided.

use serde::{Deserialize, Serialize};

use crate::digest::Digest;
use crate::error::{Result, VdsError};

/// Figma spells a node id two ways. `674-6069` in a URL, `674:6069` in the
/// document. One rule, in one place: [`crate::normalise_node_id`].
pub use crate::normalise_node_id;

/// What the Principal decided about one frame. CLOSED at three.
///
/// `defer` is a decision and not a silence: a deferred frame is coverage owed,
/// reported and never registered ([2026] VJS-FI-VDS 2 order 8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionVerdict {
    Sign,
    Refuse,
    Defer,
}

impl DecisionVerdict {
    pub fn as_str(self) -> &'static str {
        match self {
            DecisionVerdict::Sign => "sign",
            DecisionVerdict::Refuse => "refuse",
            DecisionVerdict::Defer => "defer",
        }
    }
}

impl std::fmt::Display for DecisionVerdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One row of the O2 decision export: an express Principal act about ONE frame.
///
/// `deny_unknown_fields` deliberately. The export is generated to this contract
/// and a field the door does not understand is a field the door cannot check;
/// breaking loudly when the export grows is the direction of error this door
/// wants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PrincipalDecision {
    pub schema_version: u32,
    /// The Figma file the decision was taken in.
    pub file_key: String,
    /// The decision's position in the ledger. The row cites THIS and no other.
    pub seq: u64,
    /// The frame the decision is about, in the `12:34` spelling.
    pub node_id: String,
    pub route: String,
    pub decision: DecisionVerdict,
    /// The node the Principal ADOPTED as the locus of the decision.
    ///
    /// On all 127 signed frames in the subject estate this equals `node_id`,
    /// which is why [2026] VJS-FI-VDS 2 refused to grow `SignOff` a locus
    /// field: where the locus is the frame's own node, `SignOff.node_id`
    /// already names it. Where it is NOT, the door refuses and refers - that
    /// case is expressly reserved and is not this door's to guess.
    pub locus_id: String,
    pub locus_name: String,
    /// The layer the TOOL resolved as authoritative, where it resolved one.
    /// Recorded so an override is visible on the face of the row.
    #[serde(default)]
    pub tool_proposed: Option<String>,
    /// Whether the Principal's locus differs from the tool's proposal.
    pub overrides: bool,
    /// The frame's content digest as the frames ledger computed it AT THE
    /// MOMENT OF DECISION. The door checks this against the ledger's current
    /// row: a decision taken over a drawing that has since changed is a
    /// decision about a drawing that no longer exists.
    pub frame_digest: Digest,
    /// The frames ledger's aggregate digest at the moment of decision.
    ///
    /// RECORDED and never RELIED ON. It is here so a reader can tell which
    /// ledger the signer was looking at, and the door refuses any attempt to
    /// make it the basis of a registration.
    pub ledger_digest: Digest,
    /// The export carries the source timestamp VERBATIM, in whatever form the
    /// recording tool wrote it. A `String` and not a [`crate::Timestamp`]:
    /// re-formatting a foreign record into VDS's canonical form would silently
    /// change the bytes the digest is taken over.
    pub recorded_at: String,
    pub recorded_by: String,
    /// What the signer was shown at the moment of signing, VERBATIM.
    ///
    /// Opaque to this door on purpose. It is the only surviving record of what
    /// was hidden at signing, and normalising it would destroy the evidence.
    pub disclosed_at_signing: serde_json::Value,
}

/// The O2 export's index: what was exported, and the digest of each row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DecisionExportIndex {
    pub schema_version: u32,
    pub file_key: String,
    /// What the export was taken FROM, named so a reader can go back to it.
    pub source: String,
    pub generated_at: String,
    pub row_count: u32,
    /// A digest over the whole canonical export.
    ///
    /// It exists so the export can be shown to be intact, and it is NEVER a
    /// basis for a registration. [`admit_under_principal_act`] refuses it as
    /// one by name.
    pub aggregate_digest: Digest,
    pub rows: Vec<DecisionIndexRow>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DecisionIndexRow {
    pub seq: u64,
    pub node_id: String,
    /// Repository-relative path of the exported decision file.
    pub path: String,
    /// The digest of that file's exact bytes.
    pub digest: Digest,
}

impl DecisionExportIndex {
    /// The indexed row for a sequence number.
    pub fn row(&self, seq: u64) -> Option<&DecisionIndexRow> {
        self.rows.iter().find(|r| r.seq == seq)
    }
}

/// The reference a `principal_act` sign-off row carries on its face.
///
/// [2026] VJS-FI-VDS 2 order 5. Four values, so a reader can go from the row to
/// the act without trusting the row: the decision's position, its exact bytes,
/// and the locus the Principal adopted, by id and by the name he saw.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DecisionReference {
    pub seq: u64,
    /// The digest of the decision's exact exported bytes.
    pub digest: Digest,
    pub locus_id: String,
    pub locus_name: String,
}

impl PrincipalDecision {
    /// Parse one decision from its exact exported bytes.
    ///
    /// Two stages, and the order is the point. An aggregate-scoped attestation
    /// is refused BY NAME before the typed parse ever runs, so the reader is
    /// told the document is the wrong KIND of thing rather than the wrong
    /// shape.
    pub fn parse(bytes: &[u8], path: &str) -> Result<PrincipalDecision> {
        let value: serde_json::Value = serde_json::from_slice(bytes)
            .map_err(|error| VdsError::parse(path, "a Principal decision", error))?;
        refuse_if_aggregate_scoped(&value, path)?;
        let decision: PrincipalDecision = serde_json::from_value(value)
            .map_err(|error| VdsError::parse(path, "a Principal decision", error))?;
        Ok(decision)
    }
}

/// Refuse a document whose subject is the LEDGER rather than a frame.
///
/// The shape this catches is a real one: an `external-principal-frame-signature`
/// carrying `scope.aggregateDigest` was the register's only external door
/// before this judgment, and it attests to the record that classifies the
/// frames rather than to any frame. Whether that door has some other lawful use
/// is expressly reserved; it is not a basis for a registration.
fn refuse_if_aggregate_scoped(value: &serde_json::Value, path: &str) -> Result<()> {
    let Some(object) = value.as_object() else {
        return Err(VdsError::precondition(format!(
            "{path}: a Principal decision is a JSON object naming one frame. This document is \
             not an object at all, so it names no subject."
        )));
    };
    let names_a_frame = object
        .get("nodeId")
        .and_then(|v| v.as_str())
        .is_some_and(|s| !s.trim().is_empty());
    let aggregate_field = [
        "aggregateDigest",
        "frameLedgerDigest",
        "ledgerAggregateDigest",
    ]
    .into_iter()
    .find(|key| contains_key(value, key));
    let scoped_to_a_file = object.get("scope").is_some_and(|s| s.is_object());
    let kind = object.get("kind").and_then(|v| v.as_str());
    if names_a_frame && aggregate_field.is_none() && kind != Some(AGGREGATE_KIND) {
        return Ok(());
    }
    if !names_a_frame
        && (aggregate_field.is_some() || scoped_to_a_file || kind == Some(AGGREGATE_KIND))
    {
        return Err(aggregate_refusal(path, aggregate_field, kind));
    }
    // It names a frame AND carries an aggregate scope. `ledgerDigest` on a
    // decision row is lawful and recorded; a `scope` block or an
    // `aggregateDigest` is the aggregate wearing a per-frame name.
    if aggregate_field.is_some() || kind == Some(AGGREGATE_KIND) {
        return Err(aggregate_refusal(path, aggregate_field, kind));
    }
    Ok(())
}

const AGGREGATE_KIND: &str = "external-principal-frame-signature";

fn aggregate_refusal(path: &str, field: Option<&str>, kind: Option<&str>) -> VdsError {
    let what = match (field, kind) {
        (Some(field), _) => format!("carries {field:?}"),
        (None, Some(kind)) => format!("declares kind {kind:?}"),
        (None, None) => "is scoped to a file rather than to a frame".to_owned(),
    };
    VdsError::precondition(format!(
        "{path}: this document {what}, so its subject is the FRAMES LEDGER and not a frame. \
         An attestation over the ledger as a whole attests to the record that CLASSIFIES the \
         frames; it says nothing about any one of them, and it is not a label-resolution act \
         (CC-OPBOX 7 R5, forbidden expressly by [2026] VJS-FI-VDS 2).\n  \
         Limb (b) is PER-FRAME. Pass the one exported decision for this node with --decision, \
         and its export index with --decisions-index."
    ))
}

/// Whether any object anywhere in the document carries a key.
fn contains_key(value: &serde_json::Value, key: &str) -> bool {
    match value {
        serde_json::Value::Object(map) => {
            map.contains_key(key) || map.values().any(|v| contains_key(v, key))
        }
        serde_json::Value::Array(items) => items.iter().any(|v| contains_key(v, key)),
        _ => false,
    }
}

/// LIMB (b). Whether an express Principal act admits this frame, and on what
/// terms it is recorded.
///
/// Every limb is a REFUSAL with its own words. A door that returns one message
/// for five different failures teaches a caller to retry rather than to look.
///
/// `decision_bytes` is the file's exact bytes and not a re-serialisation of the
/// parsed value: the digest the export recorded is a digest of what was
/// written, and re-serialising to check it would compare the door against
/// itself.
#[allow(clippy::too_many_arguments)]
pub fn admit_under_principal_act(
    file_key: &str,
    frame_node_id: &str,
    frame_current_digest: &Digest,
    decision_bytes: &[u8],
    decision_path: &str,
    index: &DecisionExportIndex,
    index_path: &str,
) -> Result<DecisionReference> {
    let decision = PrincipalDecision::parse(decision_bytes, decision_path)?;

    // 1. THE EXPORT SAYS THIS IS THE DECISION. Checked first, because every
    //    later limb reads fields out of these bytes, and a check over bytes
    //    nobody vouched for is a check over a caller's opinion.
    let supplied = Digest::of_bytes(decision_bytes);
    let Some(row) = index.row(decision.seq) else {
        return Err(VdsError::precondition(format!(
            "{index_path} exports {} decision(s) and none of them is seq {}. A sign-off row \
             cites ONE decision by its position in the ledger, and a decision the export does \
             not carry is one no reader can go back to ([2026] VJS-FI-VDS 2 order 4).",
            index.rows.len(),
            decision.seq
        )));
    };
    if row.digest != supplied {
        return Err(VdsError::precondition(format!(
            "{decision_path} digests to {supplied}, and {index_path} records seq {} at {}. \
             These are not the same bytes, so this is not the exported decision: it has been \
             edited, re-serialised, or authored. A decision a caller can rewrite is a decision \
             a caller can make ([2026] VJS-FI-VDS 2 order 4).",
            decision.seq, row.digest
        )));
    }
    if normalise_node_id(&row.node_id) != normalise_node_id(&decision.node_id) {
        return Err(VdsError::precondition(format!(
            "{index_path} files seq {} under node {}, and the decision itself names node {}. \
             The export disagrees with its own index, so neither can be relied on.",
            decision.seq, row.node_id, decision.node_id
        )));
    }

    // 2. IT IS ABOUT THIS FRAME.
    if normalise_node_id(&decision.node_id) != normalise_node_id(frame_node_id) {
        return Err(VdsError::precondition(format!(
            "the decision at {decision_path} (seq {}) is about {}/{}, and this door is signing \
             {file_key}/{frame_node_id}. Limb (b) is a PER-FRAME act: a decision about another \
             frame admits nothing here.",
            decision.seq, decision.file_key, decision.node_id
        )));
    }
    if decision.file_key != file_key {
        return Err(VdsError::precondition(format!(
            "the decision at {decision_path} (seq {}) was taken in Figma file {:?}, and this \
             sign-off is for {:?}. Signing across files would bind an act about one drawing to \
             a claim about another.",
            decision.seq, decision.file_key, file_key
        )));
    }

    // 3. IT SAYS SIGN, AND NOTHING ELSE COUNTS.
    if decision.decision != DecisionVerdict::Sign {
        return Err(VdsError::precondition(format!(
            "the decision at {decision_path} (seq {}) for {}/{} is {:?}, not `sign`. \
             [2026] VJS-FI-VDS 2 forbids a sign-off row citing a decision whose decision is not \
             exactly `sign`, and a refusal is the opposite of a registration: it does not create \
             a row, and where a row already binds that frame at that digest it DESTROYS one.",
            decision.seq,
            decision.file_key,
            decision.node_id,
            decision.decision.as_str()
        )));
    }

    // 4. THE LOCUS IS THE FRAME'S OWN NODE. Where it is not, the door REFUSES
    //    AND REFERS. [2026] VJS-FI-VDS 2 expressly reserves that case: a
    //    `SignOff` binds the FRAME's digest and cannot express adoption of a
    //    sub-layer, so admitting one would record a claim the row cannot make.
    //    It happened once in 167 decisions, and that one was a refusal.
    if normalise_node_id(&decision.locus_id) != normalise_node_id(&decision.node_id) {
        return Err(VdsError::precondition(format!(
            "the decision at {decision_path} (seq {}) adopts locus {} ({:?}), which is NOT the \
             frame's own node {}. This door cannot admit it and does not guess: a `SignOff` \
             binds the FRAME's content digest and has no field that can name a layer within it, \
             so a row written here would silently claim authority over the whole frame on an act \
             about part of it.\n  \
             REFER IT. [2026] VJS-FI-VDS 2 expressly RESERVES whether a Principal may adopt a \
             locus that is not the frame's own node, and what row shape that would require. \
             That question is open and is not this door's to decide.",
            decision.seq, decision.locus_id, decision.locus_name, decision.node_id
        )));
    }

    // 5. IT IS ABOUT THE FRAME AS IT NOW STANDS.
    if &decision.frame_digest != frame_current_digest {
        return Err(VdsError::precondition(format!(
            "the decision at {decision_path} (seq {}) was taken over {}/{} at content digest {}, \
             and the frames ledger now records {}. The drawing changed after the decision, so \
             the act covers a frame that no longer exists.\n  \
             Staleness is by HASH, never by trust: re-capture and put the frame to the Principal \
             again ([2026] VJS-FI-VDS 2 order 4).",
            decision.seq,
            decision.file_key,
            decision.node_id,
            decision.frame_digest,
            frame_current_digest
        )));
    }

    // 6. AND IT IS NOT THE AGGREGATE WEARING A PER-FRAME NAME.
    if decision.frame_digest == decision.ledger_digest {
        return Err(VdsError::precondition(format!(
            "the decision at {decision_path} (seq {}) records the SAME digest {} as both its \
             frame digest and its ledger digest. A frame's content digest is never the ledger's \
             aggregate, so this row is the ledger-wide attestation under a per-frame field name, \
             and an attestation over the record that CLASSIFIES the frames is not an act upon \
             any one of them (CC-OPBOX 7 R5, forbidden expressly by [2026] VJS-FI-VDS 2).",
            decision.seq, decision.frame_digest
        )));
    }
    if index.aggregate_digest == supplied {
        return Err(VdsError::precondition(format!(
            "{decision_path} digests to {index_path}'s AGGREGATE {supplied}, so what was passed \
             is the export as a whole and not a decision within it. The aggregate is not a \
             sufficient basis for a registration and this door refuses it as one \
             ([2026] VJS-FI-VDS 2 order 4)."
        )));
    }

    Ok(DecisionReference {
        seq: decision.seq,
        digest: supplied,
        locus_id: decision.locus_id,
        locus_name: decision.locus_name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decision_json(overrides: serde_json::Value) -> Vec<u8> {
        let mut value = serde_json::json!({
            "schemaVersion": 1,
            "fileKey": "KEY",
            "seq": 7,
            "nodeId": "674:6069",
            "route": "/approve/[token]",
            "decision": "sign",
            "locusId": "674:6069",
            "locusName": "Screen \u{b7} /approve/[token]",
            "toolProposed": "Screen \u{b7} /approve/[token]",
            "overrides": false,
            "frameDigest": Digest::of_text("frame-v1"),
            "ledgerDigest": Digest::of_text("ledger-v1"),
            "recordedAt": "2026-08-04T06:31:18.907274+00:00",
            "recordedBy": "tools/sign-review/sign-review-server.py",
            "disclosedAtSigning": {"authorityBy": "frame_own_children"}
        });
        for (key, replacement) in overrides.as_object().expect("an object") {
            value[key] = replacement.clone();
        }
        serde_json::to_vec(&value).expect("json")
    }

    fn index_over(bytes: &[u8]) -> DecisionExportIndex {
        let parsed: serde_json::Value = serde_json::from_slice(bytes).expect("json");
        DecisionExportIndex {
            schema_version: 1,
            file_key: "KEY".into(),
            source: "sign-decisions.sqlite".into(),
            generated_at: "2026-08-04T08:00:00Z".into(),
            row_count: 1,
            aggregate_digest: Digest::of_text("the whole export"),
            rows: vec![DecisionIndexRow {
                seq: parsed["seq"].as_u64().expect("seq"),
                node_id: parsed["nodeId"].as_str().expect("node").into(),
                path: "decisions/0007.json".into(),
                digest: Digest::of_bytes(bytes),
            }],
        }
    }

    fn admit(bytes: &[u8], index: &DecisionExportIndex) -> Result<DecisionReference> {
        admit_under_principal_act(
            "KEY",
            "674:6069",
            &Digest::of_text("frame-v1"),
            bytes,
            "decisions/0007.json",
            index,
            "decisions/INDEX.json",
        )
    }

    #[test]
    fn an_express_per_frame_sign_admits_the_frame_and_names_the_locus() {
        let bytes = decision_json(serde_json::json!({}));
        let index = index_over(&bytes);
        let reference = admit(&bytes, &index).expect("admitted");
        assert_eq!(reference.seq, 7);
        assert_eq!(reference.locus_id, "674:6069");
        assert_eq!(reference.digest, Digest::of_bytes(&bytes));
    }

    /// NEGATIVE CONTROL 1 ([2026] VJS-FI-VDS 2 order 4).
    #[test]
    fn a_decision_that_says_refuse_admits_nothing() {
        let bytes = decision_json(serde_json::json!({"decision": "refuse"}));
        let index = index_over(&bytes);
        let error = admit(&bytes, &index).unwrap_err().to_string();
        assert!(error.contains("is \"refuse\", not `sign`"), "{error}");
        assert!(error.contains("DESTROYS one"), "{error}");
    }

    #[test]
    fn a_decision_that_says_defer_admits_nothing_either() {
        let bytes = decision_json(serde_json::json!({"decision": "defer"}));
        let index = index_over(&bytes);
        let error = admit(&bytes, &index).unwrap_err().to_string();
        assert!(error.contains("is \"defer\", not `sign`"), "{error}");
    }

    /// NEGATIVE CONTROL 2 ([2026] VJS-FI-VDS 2 order 4). The index is built
    /// HONESTLY over the tampered bytes, so the integrity limb passes and the
    /// frame-digest limb is the one that fires. A control that trips an earlier
    /// check proves the earlier check, not the one it was written for.
    #[test]
    fn a_decision_whose_frame_digest_is_not_the_ledgers_admits_nothing() {
        let bytes = decision_json(serde_json::json!({
            "frameDigest": Digest::of_text("some-other-frame")
        }));
        let index = index_over(&bytes);
        let error = admit(&bytes, &index).unwrap_err().to_string();
        assert!(error.contains("changed after the decision"), "{error}");
        assert!(error.contains("never by trust"), "{error}");
    }

    /// NEGATIVE CONTROL 2b, and it exists because limb 6 was UNCOVERED. Disabling
    /// `if decision.frame_digest == decision.ledger_digest` left the whole suite
    /// green, which found by sabotage what no assertion was watching: the limb
    /// implementing CC-OPBOX 7 R5 at this door had no test at all.
    ///
    /// The aggregate wearing a per-frame field name is the ONE shape that gets
    /// past limb 5, because a decision that copies the ledger digest into
    /// `frameDigest` matches the frame's current digest whenever the caller is
    /// also handed the aggregate as the frame's digest. So the frame digest is
    /// set to the ledger digest AND the door is told that is the current digest -
    /// every earlier limb is satisfied and limb 6 is the only thing standing.
    #[test]
    fn a_decision_whose_frame_digest_is_the_ledger_aggregate_admits_nothing() {
        let aggregate = Digest::of_text("ledger-v1");
        let bytes = decision_json(serde_json::json!({ "frameDigest": aggregate }));
        let index = index_over(&bytes);
        let error = admit_under_principal_act(
            "KEY",
            "674:6069",
            &aggregate,
            &bytes,
            "decisions/0007.json",
            &index,
            "decisions/INDEX.json",
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("SAME digest"), "{error}");
        assert!(error.contains("CLASSIFIES"), "{error}");
    }

    /// NEGATIVE CONTROL 3 ([2026] VJS-FI-VDS 2 order 4): the aggregate is not a
    /// label-resolution act, and the refusal must say so by name rather than
    /// reporting a malformed decision.
    #[test]
    fn an_attestation_scoped_to_the_whole_ledger_admits_nothing() {
        let bytes = serde_json::to_vec(&serde_json::json!({
            "schemaVersion": 1,
            "kind": "external-principal-frame-signature",
            "signedBy": "Principal",
            "signedAt": "2026-08-03T16:49:22Z",
            "actualReply": "Signed off. proceed all",
            "warrant": false,
            "scope": {
                "fileKey": "KEY",
                "aggregateDigest": Digest::of_text("ledger-v1"),
            }
        }))
        .expect("json");
        let index = index_over(&decision_json(serde_json::json!({})));
        let error = admit(&bytes, &index).unwrap_err().to_string();
        assert!(error.contains("subject is the FRAMES LEDGER"), "{error}");
        assert!(error.contains("not a label-resolution act"), "{error}");
        assert!(error.contains("Limb (b) is PER-FRAME"), "{error}");
    }

    #[test]
    fn the_aggregate_is_refused_even_when_it_names_a_frame() {
        let bytes = decision_json(serde_json::json!({
            "aggregateDigest": Digest::of_text("ledger-v1")
        }));
        let index = index_over(&bytes);
        let error = admit(&bytes, &index).unwrap_err().to_string();
        assert!(error.contains("subject is the FRAMES LEDGER"), "{error}");
    }

    /// The reserved case. A locus that is not the frame's own node is REFERRED,
    /// never guessed at.
    #[test]
    fn a_locus_that_is_not_the_frames_own_node_is_refused_and_referred() {
        let bytes = decision_json(serde_json::json!({"locusId": "1007:89086"}));
        let index = index_over(&bytes);
        let error = admit(&bytes, &index).unwrap_err().to_string();
        assert!(error.contains("REFER IT"), "{error}");
        assert!(error.contains("RESERVES"), "{error}");
    }

    #[test]
    fn a_decision_about_another_frame_admits_nothing() {
        let bytes = decision_json(serde_json::json!({
            "nodeId": "999:1", "locusId": "999:1"
        }));
        let index = index_over(&bytes);
        let error = admit(&bytes, &index).unwrap_err().to_string();
        assert!(error.contains("PER-FRAME act"), "{error}");
    }

    /// An edited decision is not the exported decision, whatever it says.
    #[test]
    fn bytes_the_export_did_not_record_admit_nothing() {
        let honest = decision_json(serde_json::json!({}));
        let index = index_over(&honest);
        let edited = decision_json(serde_json::json!({"route": "/somewhere-else"}));
        let error = admit(&edited, &index).unwrap_err().to_string();
        assert!(error.contains("not the exported decision"), "{error}");
    }

    #[test]
    fn the_url_spelling_of_a_node_id_is_the_same_node() {
        let bytes = decision_json(serde_json::json!({
            "nodeId": "674-6069", "locusId": "674-6069"
        }));
        let index = index_over(&bytes);
        assert!(admit(&bytes, &index).is_ok());
    }
}
