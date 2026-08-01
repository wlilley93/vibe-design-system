//! Frame sign-offs, authority states, and proposed redraws.
//!
//! Draft S-7D, ENACTMENT PENDING (SUBMISSION-VDS-017). The constitutional
//! direction this implements: THE SIGNED-OFF FIGMA DEFINES TASTE. Taste is
//! exercised once, at frame sign-off, and deviations are not adjudicated
//! downstream. Three consequences, each carried by a type here:
//!
//!   1. [`SignOff`]: authority for any frame-bound proof exists ONLY while the
//!      frame's current content hash matches a sign-off row. Staleness is by
//!      HASH, never by trust: a frame edited after sign-off reverts to
//!      UNSIGNED until re-signed, however senior the last signer.
//!   2. [`AuthorityVerdict`]: the verdict vocabulary of every frame-bound
//!      proof is `conform | deviate | no_authority`. `no_authority` is a
//!      DISTINCT state - never green, never red, reported as coverage owed -
//!      and a proof cannot claim conformance against an unsigned frame.
//!   3. There is NO ACCEPTANCE STATE in the engine. An addition the frame
//!      omits is a deviation exactly like a missing element. The resolution
//!      path is [`RedrawRecord`]: the band comes back through the design,
//!      never through an engine-side excusal. This deliberately REPEALS the
//!      direction-blind and direction-carrying acceptance concepts; the
//!      repeal is stated in the submission rather than smuggled.
//!
//! # Why a sign-off stores a hash and not an approval bit
//!
//! An approval bit is trust, and trust does not expire when the frame changes.
//! The hash is the frame's content at the moment taste was exercised, so the
//! authority claim is checkable forever: current hash matches, the taste
//! decision still covers what the frame now shows; it moved, the decision
//! covers a drawing that no longer exists.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::digest::Digest;
use crate::ids::{RedrawId, ReviewId, SignoffId};
use crate::timestamp::Timestamp;

/// One frame sign-off: the frame's content hash at the moment it was signed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SignOff {
    pub id: SignoffId,
    pub file_key: String,
    /// The frame's node id, in the `12:34` spelling.
    pub node_id: String,
    /// The frame's content digest AT SIGN-OFF, as the frames ledger computes
    /// it. The whole mechanism: authority holds while the current digest
    /// equals this one, and not a moment longer.
    pub frame_digest: Digest,
    pub signed_by: String,
    pub signed_at: Timestamp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// Whether a frame currently carries authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameAuthority {
    /// The frame's current content hash matches a sign-off row.
    Signed { signoff: SignoffId },
    /// No sign-off covers the frame AS IT NOW IS. Either it was never signed,
    /// or it changed after sign-off; the two are distinguished in `because`
    /// and identical in effect (staleness by hash, never by trust).
    Unsigned { because: String },
}

impl FrameAuthority {
    pub fn is_signed(&self) -> bool {
        matches!(self, FrameAuthority::Signed { .. })
    }
}

/// Resolve a frame's authority from its CURRENT content digest and the
/// sign-off register.
///
/// `current` is `None` where the frame is not in the ledger or the ledger
/// predates per-frame digests. That resolves UNSIGNED, fail-closed: a frame
/// whose current content cannot be measured cannot be shown to match any
/// sign-off, and guessing "signed" there would be authority by trust.
pub fn frame_authority(
    file_key: &str,
    node_id: &str,
    current: Option<&Digest>,
    signoffs: &[SignOff],
) -> FrameAuthority {
    let rows: Vec<&SignOff> = signoffs
        .iter()
        .filter(|s| s.file_key == file_key && s.node_id == node_id)
        .collect();
    let Some(current) = current else {
        return FrameAuthority::Unsigned {
            because: format!(
                "the frame {file_key}/{node_id} has no current content digest: it is not in \
                 the frames ledger, or the ledger predates per-frame digests. A frame whose \
                 current content cannot be measured cannot be shown to match any sign-off."
            ),
        };
    };
    if let Some(matched) = rows.iter().find(|s| &s.frame_digest == current) {
        return FrameAuthority::Signed {
            signoff: matched.id.clone(),
        };
    }
    FrameAuthority::Unsigned {
        because: if rows.is_empty() {
            format!("no sign-off row exists for {file_key}/{node_id}: the frame was never signed")
        } else {
            format!(
                "{} sign-off row(s) exist for {file_key}/{node_id} and none matches the \
                 frame's current content digest {current}. The frame changed after sign-off, \
                 so authority reverted to UNSIGNED until re-signed (staleness by hash, never \
                 by trust).",
                rows.len()
            )
        },
    }
}

/// The verdict vocabulary of every frame-bound proof. CLOSED at three.
///
/// There is deliberately no fourth variant for an accepted deviation. An
/// acceptance state is taste exercised downstream of sign-off, which the
/// constitutional direction forbids: the resolution path for a deviation is a
/// new signed frame version, recorded as a [`RedrawRecord`].
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityVerdict {
    /// The shipped artefact matches the signed frame.
    Conform,
    /// The shipped artefact differs from the signed frame. An ADDITION the
    /// frame omits is a deviation exactly like a missing element; direction
    /// carries no excuse.
    Deviate,
    /// The frame is unsigned, parked, or a proposal: there is no authority to
    /// conform to. Never green, never red; reported as coverage owed.
    NoAuthority,
}

impl AuthorityVerdict {
    pub fn as_str(self) -> &'static str {
        match self {
            AuthorityVerdict::Conform => "conform",
            AuthorityVerdict::Deviate => "deviate",
            AuthorityVerdict::NoAuthority => "no_authority",
        }
    }
}

impl std::fmt::Display for AuthorityVerdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Where a proposed redraw stands. CLOSED at four; there is no "accepted".
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum RedrawStatus {
    /// The change is described and owed to the design.
    Proposed,
    /// The change has been drawn and awaits sign-off.
    Drawn,
    /// A sign-off row covering the change exists. ONLY lawful with
    /// `resolved_by` naming it; the proof refuses the word without the row.
    Signed,
    /// The proposal was withdrawn; the deviation stands and stays red.
    Withdrawn,
}

impl RedrawStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            RedrawStatus::Proposed => "proposed",
            RedrawStatus::Drawn => "drawn",
            RedrawStatus::Signed => "signed",
            RedrawStatus::Withdrawn => "withdrawn",
        }
    }
}

impl std::fmt::Display for RedrawStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A deviation routed back through the design: the machine-readable form of
/// "add it neatly later".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RedrawRecord {
    pub id: RedrawId,
    /// The deviation this resolves: a review record and the delta it named.
    pub deviation: String,
    /// The review that recorded the deviation, where one exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_id: Option<ReviewId>,
    /// The proposed design change, described. A DESCRIPTION of intent, never a
    /// value: the change itself happens in the design file.
    pub proposed: String,
    pub status: RedrawStatus,
    pub file_key: String,
    pub node_id: String,
    /// The sign-off row that resolves this redraw. `signed` without it is
    /// refused by the proof: a deviation is resolvable ONLY by a sign-off row
    /// whose hash covers the change, never by the word "signed".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_by: Option<SignoffId>,
    pub opened_at: Timestamp,
    #[serde(default)]
    pub basis: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signoff(node: &str, digest: &str) -> SignOff {
        SignOff {
            id: SignoffId::parse("SGN-0001").unwrap(),
            file_key: "KEY".into(),
            node_id: node.into(),
            frame_digest: Digest::of_text(digest),
            signed_by: "the principal".into(),
            signed_at: Timestamp::fixed(2026, 8, 1, 10, 0, 0),
            notes: None,
        }
    }

    #[test]
    fn a_matching_hash_is_signed_authority() {
        let rows = vec![signoff("1:2", "frame-v1")];
        let current = Digest::of_text("frame-v1");
        assert!(frame_authority("KEY", "1:2", Some(&current), &rows).is_signed());
    }

    /// The seed the mandate names: a signed frame whose hash then drifts must
    /// revert to UNSIGNED. Staleness by hash, never by trust.
    #[test]
    fn a_frame_that_changed_after_signoff_reverts_to_unsigned() {
        let rows = vec![signoff("1:2", "frame-v1")];
        let moved = Digest::of_text("frame-v2");
        let authority = frame_authority("KEY", "1:2", Some(&moved), &rows);
        assert!(!authority.is_signed());
        let FrameAuthority::Unsigned { because } = authority else {
            unreachable!()
        };
        assert!(because.contains("changed after sign-off"), "{because}");
        assert!(because.contains("never by trust"), "{because}");
    }

    #[test]
    fn a_never_signed_frame_is_unsigned_and_says_so_differently() {
        let current = Digest::of_text("frame-v1");
        let authority = frame_authority("KEY", "9:9", Some(&current), &[]);
        let FrameAuthority::Unsigned { because } = authority else {
            panic!("must be unsigned")
        };
        assert!(because.contains("never signed"), "{because}");
    }

    #[test]
    fn a_frame_with_no_measurable_current_content_is_unsigned_fail_closed() {
        let rows = vec![signoff("1:2", "frame-v1")];
        assert!(!frame_authority("KEY", "1:2", None, &rows).is_signed());
    }

    #[test]
    fn the_verdict_vocabulary_is_closed_at_three_and_has_no_acceptance() {
        assert!(serde_json::from_str::<AuthorityVerdict>("\"accepted\"").is_err());
        assert!(serde_json::from_str::<AuthorityVerdict>("\"no_authority\"").is_ok());
        assert!(serde_json::from_str::<RedrawStatus>("\"accepted\"").is_err());
    }
}
