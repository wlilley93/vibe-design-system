//! Warrants: the four stage gates.
//!
//! A warrant is an operative record of what was granted, on what evidence
//! digest, by whom, when, and what it unlocks (VDS S-6(1)). VDS does not grant
//! one. W1, W2 and W4 are VJS's on a referred submission and W3 is the
//! Principal's alone (VDS S-1(3), S-6(7)), so everything in this module records
//! a grant that happened elsewhere and pins what it was made on.
//!
//! VDS S-6(3): a warrant carrying no evidence entry is a signature on nothing
//! and is void on its face. [`Warrant::void_on_its_face`] is that test, applied
//! to the record rather than asserted about it.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{ProofKind, ProofStatus};
use crate::digest::Digest;
use crate::ids::{ProofId, WarrantId};
use crate::timestamp::Timestamp;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
pub enum Stage {
    #[serde(rename = "W1_REGISTER_COMPLETE")]
    W1RegisterComplete,
    #[serde(rename = "W2_DESIGN_COMPLETE")]
    W2DesignComplete,
    #[serde(rename = "W3_PRINCIPAL_ACCEPTED")]
    W3PrincipalAccepted,
    #[serde(rename = "W4_PARITY")]
    W4Parity,
}

impl Stage {
    pub const ALL: [Stage; 4] = [
        Stage::W1RegisterComplete,
        Stage::W2DesignComplete,
        Stage::W3PrincipalAccepted,
        Stage::W4Parity,
    ];

    pub fn number(self) -> u8 {
        match self {
            Stage::W1RegisterComplete => 1,
            Stage::W2DesignComplete => 2,
            Stage::W3PrincipalAccepted => 3,
            Stage::W4Parity => 4,
        }
    }

    pub fn short(self) -> &'static str {
        match self {
            Stage::W1RegisterComplete => "W1",
            Stage::W2DesignComplete => "W2",
            Stage::W3PrincipalAccepted => "W3",
            Stage::W4Parity => "W4",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Stage::W1RegisterComplete => "W1_REGISTER_COMPLETE",
            Stage::W2DesignComplete => "W2_DESIGN_COMPLETE",
            Stage::W3PrincipalAccepted => "W3_PRINCIPAL_ACCEPTED",
            Stage::W4Parity => "W4_PARITY",
        }
    }

    /// Accepts either the short form (`W1`) or the full stage name.
    pub fn parse(raw: &str) -> Option<Stage> {
        Stage::ALL
            .into_iter()
            .find(|s| s.short().eq_ignore_ascii_case(raw) || s.as_str() == raw)
    }

    /// The proof kinds this stage is granted on. VDS S-6(2).
    pub fn required_evidence(self) -> &'static [ProofKind] {
        match self {
            Stage::W1RegisterComplete => {
                &[ProofKind::RegisterCompleteness, ProofKind::Reconciliation]
            }
            Stage::W2DesignComplete => &[
                ProofKind::Composition,
                ProofKind::States,
                ProofKind::Contrast,
            ],
            // No proof substitutes for acceptance (VDS S-6(7)).
            Stage::W3PrincipalAccepted => &[],
            Stage::W4Parity => &[ProofKind::Parity, ProofKind::TokenPin, ProofKind::Contrast],
        }
    }

    pub fn unlocks(self) -> &'static str {
        match self {
            Stage::W1RegisterComplete => "design_may_begin",
            Stage::W2DesignComplete => "principal_review",
            Stage::W3PrincipalAccepted => "parity_work_may_begin",
            Stage::W4Parity => "system_complete",
        }
    }

    /// The stage that must be `granted` before this one may be entered.
    /// VDS S-6(2): the ordering is the entire mechanism.
    pub fn predecessor(self) -> Option<Stage> {
        match self {
            Stage::W1RegisterComplete => None,
            Stage::W2DesignComplete => Some(Stage::W1RegisterComplete),
            Stage::W3PrincipalAccepted => Some(Stage::W2DesignComplete),
            Stage::W4Parity => Some(Stage::W3PrincipalAccepted),
        }
    }
}

impl std::fmt::Display for Stage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WarrantStatus {
    Granted,
    Refused,
    /// The surface it was granted over changed (VDS S-6(4)). Recorded, never
    /// deleted.
    Spent,
    Superseded,
    Revoked,
}

impl WarrantStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            WarrantStatus::Granted => "granted",
            WarrantStatus::Refused => "refused",
            WarrantStatus::Spent => "spent",
            WarrantStatus::Superseded => "superseded",
            WarrantStatus::Revoked => "revoked",
        }
    }
}

impl std::fmt::Display for WarrantStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GrantedBy {
    VjsCourt,
    Principal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AssentSource {
    SovereignAssent,
    StandingBoundedAssent,
    PrincipalAcceptance,
}

/// One proof a warrant rests on.
///
/// `digest` is copied from the record on disk and never from a caller: a warrant
/// that cites a digest the caller supplied proves the caller.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceEntry {
    pub proof_id: ProofId,
    pub kind: ProofKind,
    pub digest: Digest,
    pub status: ProofStatus,
}

/// The surface a warrant was granted over. VDS S-6(4): a change to either digest
/// spends the warrant.
///
/// Both digests are computed from LIVE state, never read back from a generated
/// artefact. A surface digest taken from a ledger would go stale exactly when a
/// screen changed and the ledger was not regenerated, which is the one moment it
/// has to be right.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Surface {
    pub screens_digest: Digest,
    pub register_digest: Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceEvent {
    pub path: String,
    pub digest: Digest,
    pub accepted_at: Timestamp,
    pub accepted_by: String,
    /// The digest of exactly what was accepted, so a later claim that a screen
    /// was accepted is checkable against the bytes (VDS S-6(7)).
    pub surface_digest: Digest,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Warrant {
    pub id: WarrantId,
    pub stage: Stage,
    pub project: String,
    pub status: WarrantStatus,
    pub issue: String,
    pub holding: String,
    pub granted_by: GrantedBy,
    /// The VJS order that granted it. Null only for W3, which is the
    /// Principal's and has an acceptance event instead.
    #[serde(default)]
    pub grantor_citation: Option<String>,
    pub assent_source: AssentSource,
    #[serde(default)]
    pub acceptance_event: Option<AcceptanceEvent>,
    pub evidence: Vec<EvidenceEntry>,
    /// The sha256 of the case file the grant was made on. Repeated verbatim from
    /// the convening record, so what was decided on is provable after the fact
    /// (VDS S-10(5)).
    pub case_file_digest: Digest,
    pub directives: Vec<String>,
    pub forbidden: Vec<String>,
    #[serde(default)]
    pub exceptions: Option<String>,
    pub supersedes: Vec<WarrantId>,
    pub unlocks: Vec<String>,
    #[serde(default)]
    pub surface: Option<Surface>,
    pub runtime_summary: String,
    pub created_at: Timestamp,
    #[serde(default)]
    pub granted_at: Option<Timestamp>,
    pub bench: Vec<String>,
    #[serde(default)]
    pub vote: Option<String>,
    #[serde(default)]
    pub source_opinion: Option<String>,
    pub appealable: bool,
    /// Every reserved clause this warrant relies on. VDS S-9(10) requires a
    /// warrant relying on the informational-bare-elements interim to say so.
    pub reserved: Vec<String>,
}

impl Warrant {
    /// VDS S-6(3): a warrant carrying no evidence entry is a signature on
    /// nothing.
    ///
    /// W3 is the exception and is not an exception to the principle: its
    /// evidence is an acceptance event rather than a proof, and a W3 with
    /// neither is just as void.
    pub fn void_on_its_face(&self) -> Option<&'static str> {
        match self.stage {
            Stage::W3PrincipalAccepted if self.acceptance_event.is_none() => Some(
                "W3 carries no acceptance event. Acceptance is reserved to the Sovereign, no \
                 proof substitutes for it, and VDS may never infer it from silence \
                 (VDS S-6(7)).",
            ),
            Stage::W3PrincipalAccepted => None,
            _ if self.evidence.is_empty() => Some(
                "the warrant names no evidence. A warrant carrying no evidence entry is a \
                 signature on nothing and is void on its face (VDS S-6(3)).",
            ),
            _ => None,
        }
    }

    /// Whether the live surface still matches what this warrant was granted
    /// over. VDS S-6(4).
    pub fn is_spent_by(&self, live: &Surface) -> bool {
        match (&self.surface, self.status) {
            (Some(granted), WarrantStatus::Granted) => granted != live,
            _ => false,
        }
    }

    /// The proof kinds VDS S-6(2) requires for this stage that the warrant does
    /// not name.
    pub fn missing_evidence_kinds(&self) -> Vec<ProofKind> {
        self.stage
            .required_evidence()
            .iter()
            .copied()
            .filter(|kind| !self.evidence.iter().any(|e| e.kind == *kind))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn warrant(stage: Stage) -> Warrant {
        Warrant {
            id: WarrantId::parse(format!("WARRANT-W{}-001", stage.number())).unwrap(),
            stage,
            project: "demo".into(),
            status: WarrantStatus::Granted,
            issue: "issue".into(),
            holding: "holding".into(),
            granted_by: GrantedBy::VjsCourt,
            grantor_citation: Some("[2026] VJS-CC-DEMO 1".into()),
            assent_source: AssentSource::SovereignAssent,
            acceptance_event: None,
            evidence: vec![],
            case_file_digest: Digest::of_text("case"),
            directives: vec![],
            forbidden: vec![],
            exceptions: None,
            supersedes: vec![],
            unlocks: vec![stage.unlocks().into()],
            surface: None,
            runtime_summary: "summary".into(),
            created_at: Timestamp::fixed(2026, 7, 25, 10, 0, 0),
            granted_at: Some(Timestamp::fixed(2026, 7, 25, 10, 0, 0)),
            bench: vec!["a-judge".into()],
            vote: None,
            source_opinion: None,
            appealable: true,
            reserved: vec![],
        }
    }

    #[test]
    fn a_warrant_with_no_evidence_is_void_on_its_face() {
        assert!(
            warrant(Stage::W1RegisterComplete)
                .void_on_its_face()
                .is_some()
        );
    }

    #[test]
    fn a_w3_with_no_acceptance_event_is_void_on_its_face() {
        assert!(
            warrant(Stage::W3PrincipalAccepted)
                .void_on_its_face()
                .is_some()
        );
    }

    #[test]
    fn a_w3_needs_no_proof_evidence() {
        let mut w = warrant(Stage::W3PrincipalAccepted);
        w.acceptance_event = Some(AcceptanceEvent {
            path: "designpack/v1/provenance/assent/a.yaml".into(),
            digest: Digest::of_text("a"),
            accepted_at: Timestamp::fixed(2026, 7, 25, 10, 0, 0),
            accepted_by: "the Principal".into(),
            surface_digest: Digest::of_text("s"),
        });
        assert!(w.void_on_its_face().is_none());
        assert!(w.missing_evidence_kinds().is_empty());
    }

    #[test]
    fn stage_evidence_requirements_follow_the_specification() {
        assert_eq!(
            Stage::W1RegisterComplete.required_evidence(),
            &[ProofKind::RegisterCompleteness, ProofKind::Reconciliation]
        );
        assert_eq!(
            Stage::W2DesignComplete.required_evidence(),
            &[
                ProofKind::Composition,
                ProofKind::States,
                ProofKind::Contrast
            ]
        );
        assert!(Stage::W3PrincipalAccepted.required_evidence().is_empty());
        assert_eq!(
            Stage::W4Parity.required_evidence(),
            &[ProofKind::Parity, ProofKind::TokenPin, ProofKind::Contrast]
        );
    }

    #[test]
    fn the_stage_chain_is_a_directed_path() {
        assert_eq!(Stage::W1RegisterComplete.predecessor(), None);
        assert_eq!(
            Stage::W4Parity.predecessor(),
            Some(Stage::W3PrincipalAccepted)
        );
    }

    #[test]
    fn a_surface_change_spends_a_granted_warrant() {
        let granted = Surface {
            screens_digest: Digest::of_text("s1"),
            register_digest: Digest::of_text("r1"),
        };
        let moved = Surface {
            screens_digest: Digest::of_text("s2"),
            register_digest: Digest::of_text("r1"),
        };
        let mut w = warrant(Stage::W1RegisterComplete);
        w.surface = Some(granted.clone());
        assert!(!w.is_spent_by(&granted));
        assert!(w.is_spent_by(&moved));
    }

    #[test]
    fn an_already_spent_warrant_is_not_spent_again() {
        let mut w = warrant(Stage::W1RegisterComplete);
        w.surface = Some(Surface {
            screens_digest: Digest::of_text("s1"),
            register_digest: Digest::of_text("r1"),
        });
        w.status = WarrantStatus::Spent;
        assert!(!w.is_spent_by(&Surface {
            screens_digest: Digest::of_text("s2"),
            register_digest: Digest::of_text("r2"),
        }));
    }

    #[test]
    fn stage_parses_both_spellings() {
        assert_eq!(Stage::parse("W1"), Some(Stage::W1RegisterComplete));
        assert_eq!(Stage::parse("w1"), Some(Stage::W1RegisterComplete));
        assert_eq!(
            Stage::parse("W1_REGISTER_COMPLETE"),
            Some(Stage::W1RegisterComplete)
        );
        assert_eq!(Stage::parse("W5"), None);
    }
}
