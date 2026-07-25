//! The decided-target ledger: what the Figma file actually contains.
//!
//! [2026] VJS-CC-OPBOX 3 D1 makes the decided-target Figma file the system of
//! record for **what is decided**. VDS reads it and never overrules it, and it
//! reads it into a LEDGER rather than into a proof, for one reason that is not
//! negotiable: VDS S-7(2)(1) forbids a network call inside a proof. A proof that
//! calls Figma is not re-runnable, not deterministic and not a proof.
//!
//! So the shape is: `vds figma pull` (a ledger generator, network allowed, run
//! out of band) writes this file, and the proofs read it (deterministic, offline)
//! and refuse it when it is stale. That is the same arrangement the screens
//! ledger already uses, and it is the only arrangement that lets a proof reach
//! Figma at all.
//!
//! **This ledger holds no realisation** (VDS S-2(2)). It records NAMES, node
//! ids, variant property names and variant values, and whether a node resolved.
//! It does not record a colour, a length, a font, a duration or an easing curve,
//! and there is no field here one could be put in. What a Figma node LOOKS like
//! stays in Figma, which is where CC-OPBOX 3 D1 put it.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use vds_core::{ComponentId, Digest, Result, State, Timestamp, VdsError};

pub const LEDGER_SCHEMA_VERSION: u32 = 1;
pub const GENERATOR_COMMAND: &str = "vds figma pull";

/// One registered component's counterpart in the decided-target file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FigmaNodeRow {
    pub component_id: ComponentId,
    /// The node id the register claims.
    pub node_id: String,
    /// Whether that node id resolved in the pinned file. This is limb (c) of
    /// VDS S-5(6), which no offline check could ever answer.
    pub resolved: bool,
    /// The node's name in Figma, where it resolved. A NAME, not a value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub figma_name: Option<String>,
    /// Whether the node is a component set, meaning it carries variants.
    #[serde(default)]
    pub is_component_set: bool,
    /// Variant property names and their declared values, e.g.
    /// `State: [Default, Hover, Focus]`. Names and values only: a variant value
    /// is a label a designer typed, not a design value.
    #[serde(default)]
    pub variant_properties: BTreeMap<String, Vec<String>>,
    /// The nine states this node actually draws, derived from its variants.
    ///
    /// This is the field that makes the register stop rotting. VDS S-5(5) says
    /// a hand-maintained register decays; `states.drawn` is hand-maintained, and
    /// this is the same claim MEASURED from the file that decides it.
    #[serde(default)]
    pub states_drawn: Vec<State>,
    /// Why the row could not be completed, where it could not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unresolved_because: Option<String>,
}

/// A node present in the file that no register record claims.
///
/// The other direction of VDS S-5(6): a component drawn in the decided-target
/// file and absent from the register is a component design has committed to and
/// governance has never seen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnclaimedNode {
    pub node_id: String,
    pub figma_name: String,
    pub is_component_set: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FigmaLedger {
    pub schema_version: u32,
    /// Excluded from `content_digest`, so re-pulling an unchanged file does not
    /// move a digest any proof cites.
    pub generated_at: Timestamp,
    pub generated_by: String,
    /// The decided-target file.
    pub file_key: String,
    /// Figma's own version id for the file when it was read. This is the digest
    /// of "what is decided": VDS does not compute it, it records what the system
    /// of record says.
    pub file_version: String,
    pub file_name: String,
    pub content_digest: Digest,
    pub nodes: Vec<FigmaNodeRow>,
    /// Component sets in the file that no register record claims.
    #[serde(default)]
    pub unclaimed: Vec<UnclaimedNode>,
    /// What the pull could not see, in the words a reader needs.
    #[serde(default)]
    pub notes: Vec<String>,
}

impl FigmaLedger {
    pub fn compute_content_digest(&self) -> Result<Digest> {
        #[derive(Serialize)]
        struct Content<'a> {
            schema_version: u32,
            generated_by: &'a str,
            file_key: &'a str,
            file_version: &'a str,
            file_name: &'a str,
            nodes: &'a [FigmaNodeRow],
            unclaimed: &'a [UnclaimedNode],
        }
        Digest::of_value(&Content {
            schema_version: self.schema_version,
            generated_by: &self.generated_by,
            file_key: &self.file_key,
            file_version: &self.file_version,
            file_name: &self.file_name,
            nodes: &self.nodes,
            unclaimed: &self.unclaimed,
        })
    }

    pub fn row(&self, id: &ComponentId) -> Option<&FigmaNodeRow> {
        self.nodes.iter().find(|n| &n.component_id == id)
    }

    pub fn unresolved(&self) -> impl Iterator<Item = &FigmaNodeRow> {
        self.nodes.iter().filter(|n| !n.resolved)
    }
}

/// Map a Figma variant value onto one of the nine states, or nothing.
///
/// Deliberately conservative. A variant value is free text a designer typed, and
/// guessing that "Pressed" means `active` would let the ledger claim a state is
/// drawn on the strength of a synonym VDS invented. Only the nine names are
/// recognised, case-insensitively, plus the small set of spellings that are the
/// SAME word rather than a synonym.
///
/// Where a value is not recognised it is recorded in `variant_properties` and
/// contributes no state, and the run says how many went unmapped, so a design
/// system that names its states differently sees a number rather than a silence.
pub fn state_from_variant_value(value: &str) -> Option<State> {
    let normalised: String = value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    match normalised.as_str() {
        "default" | "rest" | "normal" | "none" => Some(State::Default),
        "hover" | "hovered" => Some(State::Hover),
        "focus" | "focused" | "focusvisible" => Some(State::Focus),
        "active" => Some(State::Active),
        "selected" => Some(State::Selected),
        "disabled" => Some(State::Disabled),
        "loading" => Some(State::Loading),
        "error" => Some(State::Error),
        "success" => Some(State::Success),
        _ => None,
    }
}

/// Whether a variant property name is the one that carries states.
///
/// Figma files name it inconsistently, and the alternative to a small list is
/// scanning every property's values for anything state-shaped, which would let a
/// `Size` property with a value called `Default` contribute a drawn state.
pub fn is_state_property(name: &str) -> bool {
    let normalised: String = name
        .trim()
        .to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    matches!(
        normalised.as_str(),
        "state" | "states" | "interaction" | "status"
    )
}

/// The states a node draws, from its variant properties.
pub fn states_from_variants(properties: &BTreeMap<String, Vec<String>>) -> (Vec<State>, usize) {
    let mut drawn = Vec::new();
    let mut unmapped = 0usize;
    for (name, values) in properties {
        if !is_state_property(name) {
            continue;
        }
        for value in values {
            match state_from_variant_value(value) {
                Some(state) if !drawn.contains(&state) => drawn.push(state),
                Some(_) => {}
                None => unmapped += 1,
            }
        }
    }
    // Specification order, so two pulls of one file never disagree about
    // presentation.
    let ordered: Vec<State> = State::ALL
        .into_iter()
        .filter(|s| drawn.contains(s))
        .collect();
    (ordered, unmapped)
}

/// Why the ledger cannot be relied on.
pub fn check_fresh(ledger: &FigmaLedger, expected_file_key: Option<&str>) -> Result<()> {
    if ledger.schema_version > LEDGER_SCHEMA_VERSION {
        return Err(VdsError::SchemaVersionAhead {
            path: "the figma ledger".into(),
            kind: "figma ledger",
            found: ledger.schema_version,
            understood: LEDGER_SCHEMA_VERSION,
        });
    }
    if ledger.content_digest != ledger.compute_content_digest()? {
        return Err(VdsError::precondition(
            "the figma ledger's content digest does not match its own contents, so it was \
             edited after it was generated. A ledger is a generated inventory and never \
             hand-edited (VDS S-4(2)).\n  Regenerate with: vds figma pull",
        ));
    }
    if let Some(expected) = expected_file_key
        && ledger.file_key != expected
    {
        return Err(VdsError::precondition(format!(
            "the figma ledger was pulled from file {:?} and the register names {:?}. Two \
             decided-target files is two opinions about what is decided.\n  \
             Regenerate with: vds figma pull",
            ledger.file_key, expected
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nine_state_names_map_and_a_synonym_does_not() {
        for state in State::ALL {
            assert_eq!(state_from_variant_value(state.as_str()), Some(state));
        }
        assert_eq!(state_from_variant_value("Hover"), Some(State::Hover));
        assert_eq!(
            state_from_variant_value("focus-visible"),
            Some(State::Focus)
        );
        assert_eq!(
            state_from_variant_value("Pressed"),
            None,
            "guessing that Pressed means active would let the ledger claim a state is drawn \
             on the strength of a synonym VDS invented"
        );
        assert_eq!(state_from_variant_value("Large"), None);
    }

    #[test]
    fn only_a_state_property_contributes_states() {
        let mut properties = BTreeMap::new();
        properties.insert(
            "Size".to_owned(),
            vec!["Default".to_owned(), "Large".to_owned()],
        );
        let (drawn, unmapped) = states_from_variants(&properties);
        assert!(
            drawn.is_empty(),
            "a Size property with a value called Default must not contribute a drawn state"
        );
        assert_eq!(unmapped, 0);
    }

    #[test]
    fn a_state_property_contributes_in_specification_order() {
        let mut properties = BTreeMap::new();
        properties.insert(
            "State".to_owned(),
            vec!["Focus".to_owned(), "Default".to_owned(), "Hover".to_owned()],
        );
        let (drawn, unmapped) = states_from_variants(&properties);
        assert_eq!(drawn, vec![State::Default, State::Hover, State::Focus]);
        assert_eq!(unmapped, 0);
    }

    #[test]
    fn an_unrecognised_state_value_is_counted_rather_than_guessed() {
        let mut properties = BTreeMap::new();
        properties.insert(
            "State".to_owned(),
            vec!["Default".to_owned(), "Squished".to_owned()],
        );
        let (drawn, unmapped) = states_from_variants(&properties);
        assert_eq!(drawn, vec![State::Default]);
        assert_eq!(
            unmapped, 1,
            "a design system naming states differently sees a number"
        );
    }

    fn ledger() -> FigmaLedger {
        let mut ledger = FigmaLedger {
            schema_version: LEDGER_SCHEMA_VERSION,
            generated_at: Timestamp::fixed(2026, 7, 25, 10, 0, 0),
            generated_by: GENERATOR_COMMAND.into(),
            file_key: "KEY".into(),
            file_version: "123456".into(),
            file_name: "Decided target".into(),
            content_digest: Digest::of_text("placeholder"),
            nodes: vec![FigmaNodeRow {
                component_id: ComponentId::parse("CMP-0001").unwrap(),
                node_id: "12:34".into(),
                resolved: true,
                figma_name: Some("Button".into()),
                is_component_set: true,
                variant_properties: BTreeMap::new(),
                states_drawn: vec![State::Default],
                unresolved_because: None,
            }],
            unclaimed: vec![],
            notes: vec![],
        };
        ledger.content_digest = ledger.compute_content_digest().unwrap();
        ledger
    }

    #[test]
    fn the_content_digest_excludes_generated_at() {
        let mut a = ledger();
        let before = a.compute_content_digest().unwrap();
        a.generated_at = Timestamp::fixed(2000, 1, 1, 0, 0, 0);
        assert_eq!(before, a.compute_content_digest().unwrap());
    }

    #[test]
    fn a_fresh_ledger_passes() {
        check_fresh(&ledger(), Some("KEY")).unwrap();
    }

    #[test]
    fn a_hand_edited_ledger_is_refused() {
        let mut edited = ledger();
        edited.nodes[0].resolved = false;
        let error = check_fresh(&edited, None).unwrap_err();
        assert!(error.to_string().contains("was edited"), "{error}");
    }

    #[test]
    fn a_ledger_from_another_file_is_refused() {
        let error = check_fresh(&ledger(), Some("OTHER")).unwrap_err();
        assert!(error.to_string().contains("two opinions"), "{error}");
    }

    /// VDS S-2(2). There is no field here a realisation could live in, and this
    /// test holds the serialised form to that.
    #[test]
    fn a_serialised_ledger_names_no_realisation() {
        let text = serde_yaml::to_string(&ledger()).unwrap();
        for forbidden in [
            "colour",
            "color",
            "fill",
            "stroke",
            "fontFamily",
            "fontSize",
            "cornerRadius",
            "opacity",
            "effect",
            "#",
        ] {
            assert!(
                !text.contains(forbidden),
                "the figma ledger serialises {forbidden:?}, which is a realisation and belongs \
                 in the Figma file, not in .vds/ (VDS S-2(2)): {text}"
            );
        }
    }
}
