//! The visual review verdict: automated eyes, recorded.
//!
//! Draft S-7D, ENACTMENT PENDING (SUBMISSION-VDS-016). The defect this closes
//! is the founding one of this whole lane: a migration shipped
//! structurally-green pages that looked nothing like their frames, under
//! twenty-eight source-side gates of which none read the ARTEFACT against the
//! FRAME. This record is the verdict artefact of an agent visual pass: what was
//! looked at, by whom, what differed, and the hashes that make the verdict
//! expire the moment either side moves.
//!
//! # What the ENGINE owns, and what it does not
//!
//! The capture and review pipeline - rendering the route, screenshotting it,
//! exporting the frame image, running the reviewing agent - lives in the
//! consuming repo. VDS S-7(2)(1) forbids a network call (or a model call)
//! inside a proof, so the engine's whole jurisdiction is the RECORD: validate
//! it, store it, and stale it. A verdict is evidence only while:
//!
//!   - the shipped side's source hash still matches the screens ledger, and
//!   - the frame side's hash still matches the frames ledger, and
//!   - the frame carries authority: a sign-off row matching its CURRENT hash.
//!
//! Any of the three moving does not degrade the verdict, it ENDS it, visibly.
//! A silently green stale review is the exact instrument failure the geometry
//! amendment recorded as 561-pin-561.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::signoff::AuthorityVerdict;
use crate::digest::Digest;
use crate::ids::ReviewId;
use crate::timestamp::Timestamp;

/// One recorded visual pass over one route, against one frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct VisualReviewRecord {
    pub id: ReviewId,
    /// The route reviewed, as the screens ledger spells it.
    pub route: String,
    pub file_key: String,
    /// The frame reviewed against, in the `12:34` spelling.
    pub node_id: String,
    /// Hash of the shipped screenshot artefact itself.
    pub shipped_screenshot_digest: Digest,
    /// The screens-ledger digest of the route's SOURCE at review time. The
    /// shipped side's input hash: when the route's source moves, this no
    /// longer matches the ledger and the verdict is STALE.
    pub shipped_source_digest: Digest,
    /// Hash of the exported frame image the reviewer saw.
    pub frame_image_digest: Digest,
    /// The frame's content digest at review time, as the frames ledger
    /// computes it. The frame side's input hash.
    pub frame_digest: Digest,
    /// The reviewer's verdict. `no_authority` is recordable - a pipeline that
    /// found the frame unsigned says so honestly - and the proof also DERIVES
    /// authority itself; a recorded `conform` against a frame the register
    /// shows unsigned is refused, not trusted.
    pub verdict: AuthorityVerdict,
    /// What differed, one delta per line. Required non-empty for `deviate`
    /// and required EMPTY for `conform`: a verdict and its evidence may not
    /// disagree.
    #[serde(default)]
    pub deltas: Vec<String>,
    /// The reviewing agent's identity: model, harness, prompt version.
    pub reviewed_by: String,
    pub reviewed_at: Timestamp,
    #[serde(default)]
    pub basis: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_review_record_round_trips_and_refuses_unknown_fields() {
        let record = VisualReviewRecord {
            id: ReviewId::parse("VRW-0001").unwrap(),
            route: "app/dash/page.tsx".into(),
            file_key: "KEY".into(),
            node_id: "1:2".into(),
            shipped_screenshot_digest: Digest::of_text("png"),
            shipped_source_digest: Digest::of_text("row"),
            frame_image_digest: Digest::of_text("frame-png"),
            frame_digest: Digest::of_text("frame"),
            verdict: AuthorityVerdict::Deviate,
            deltas: vec!["the dotfield renders behind the main area".into()],
            reviewed_by: "claude-fable-5 visual pass v1".into(),
            reviewed_at: Timestamp::fixed(2026, 8, 1, 10, 0, 0),
            basis: vec!["draft S-7D".into()],
            notes: None,
        };
        let text = serde_yaml::to_string(&record).unwrap();
        assert_eq!(
            serde_yaml::from_str::<VisualReviewRecord>(&text).unwrap(),
            record
        );
        let with_surprise = format!("{text}surprise: 1\n");
        assert!(serde_yaml::from_str::<VisualReviewRecord>(&with_surprise).is_err());
    }
}
