//! The component record: one registered component, its contract and its lineage.
//!
//! VDS S-5(1): the register is what turns "confine design to the library" from a
//! wish into a checkable condition. Every field here exists because a proof reads
//! it; a field no proof reads is a field that rots.
//!
//! Nothing in this type may hold a realisation (VDS S-2(4)). `min_ratio: 3.0` is
//! a REQUIREMENT drawn from WCAG 2.2 SC 1.4.11 and is lawful. A colour, a length,
//! a radius, a font or a duration is a realisation and has no field to live in.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{State, Status};
use crate::ids::{ComponentId, ProofId, WarrantId};
use crate::timestamp::Timestamp;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ComponentRecord {
    pub id: ComponentId,
    pub name: String,
    pub status: Status,
    /// Bumped on every amendment, so a contract change is a versioned event and
    /// never a silent edit (VDS S-9(2)).
    pub contract_version: u32,
    /// The node in the decided-target Figma file. Null only while unproposed or
    /// unresolved; a proof reports the absence rather than assuming it away.
    pub figma: Option<FigmaNode>,
    /// The built counterpart. Null while unbuilt.
    pub code: Option<CodeCounterpart>,
    pub props: Vec<PropContract>,
    pub states: StateContract,
    pub a11y: Accessibility,
    pub demand: Demand,
    /// Predecessors this component absorbed. An array because a successor may
    /// absorb several (VDS S-9(6)(3)).
    pub supersedes: Vec<ComponentId>,
    pub superseded_by: Option<ComponentId>,
    pub amendments: Vec<Amendment>,
    /// The authorities this registration rests on.
    pub basis: Vec<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deprecated_at: Option<Timestamp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retired_at: Option<Timestamp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retirement_proof_id: Option<ProofId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl ComponentRecord {
    /// The states required but not drawn, in specification order.
    pub fn required_not_drawn(&self) -> Vec<State> {
        State::ALL
            .into_iter()
            .filter(|s| self.states.required.contains(s) && !self.states.drawn.contains(s))
            .collect()
    }

    /// The states required but not built, in specification order.
    pub fn required_not_built(&self) -> Vec<State> {
        State::ALL
            .into_iter()
            .filter(|s| self.states.required.contains(s) && !self.states.built.contains(s))
            .collect()
    }

    pub fn import_path(&self) -> Option<&str> {
        self.code.as_ref().map(|c| c.import_path.as_str())
    }

    pub fn export_name(&self) -> Option<&str> {
        self.code.as_ref().map(|c| c.export_name.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct FigmaNode {
    pub file_key: String,
    /// A Figma node id, which is `<digits>:<digits>` in a file URL and
    /// `<digits>-<digits>` in a deep link. Both spellings are accepted because
    /// both are what a designer actually copies.
    pub node_id: String,
    pub captured_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CodeCounterpart {
    /// The module specifier a screen imports from, as written in the import.
    pub import_path: String,
    /// Repository-relative path, no leading slash.
    pub source_file: String,
    pub export_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PropContract {
    pub name: String,
    /// The TypeScript type expression, or a closed union written `a|b|c`.
    ///
    /// Serialised as `type`, which is the field name the contract publishes and
    /// a reserved word in Rust. The rename keeps the published shape rather than
    /// letting an implementation-language constraint leak into the artefact.
    #[serde(rename = "type")]
    pub type_expr: String,
    pub required: bool,
    /// The corresponding Figma variant property, or null where the prop has no
    /// visual counterpart.
    #[serde(default)]
    pub figma_property: Option<String>,
}

/// Which of the nine states are required, which are drawn, which are built.
///
/// Stored as sorted, deduplicated vectors rather than sets so the serialised
/// form is stable: a record's bytes must not depend on hash iteration order, or
/// the register digest moves without the register moving.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StateContract {
    pub required: Vec<State>,
    pub drawn: Vec<State>,
    pub built: Vec<State>,
}

impl StateContract {
    /// Normalise every bucket into specification order with duplicates removed.
    pub fn normalised(mut self) -> Self {
        for bucket in [&mut self.required, &mut self.drawn, &mut self.built] {
            let present: Vec<State> = State::ALL
                .into_iter()
                .filter(|s| bucket.contains(s))
                .collect();
            *bucket = present;
        }
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Accessibility {
    /// ARIA role, or null where the component is a layout container with none.
    pub role: Option<String>,
    pub accessible_name_source: NameSource,
    pub keyboard: Vec<KeyboardContract>,
    pub contrast_floors: Vec<ContrastFloor>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NameSource {
    Children,
    /// The HTML `<label>` association: a `for`/`id` pair, or a control wrapped
    /// by its label element.
    ///
    /// Added because the set without it could not describe a text input, which
    /// is the single most common named control there is. `aria_labelledby` is a
    /// DIFFERENT mechanism, not a spelling of this one: it points at arbitrary
    /// elements by id and overrides the label association, so recording one
    /// where the code does the other publishes a contract the code does not
    /// keep. Found by adopting VDS on `examples/storefront`, where the only
    /// honest value for TextField was absent from the enum.
    Label,
    AriaLabel,
    AriaLabelledby,
    Title,
    Alt,
    NoneDecorative,
}

impl NameSource {
    /// Every variant, in the order a report lists them.
    ///
    /// The CLI parses against THIS rather than against a hand-written match, and
    /// its "not an accessible-name source" message is built from it. A
    /// hand-written parse is a second list, and a second list drifts: `label` was
    /// added to the enum and the parse would have gone on refusing it while the
    /// error text went on saying "the six are", naming a set that was no longer
    /// the set.
    pub const ALL: [NameSource; 7] = [
        NameSource::Children,
        NameSource::Label,
        NameSource::AriaLabel,
        NameSource::AriaLabelledby,
        NameSource::Title,
        NameSource::Alt,
        NameSource::NoneDecorative,
    ];

    pub fn parse(raw: &str) -> Option<NameSource> {
        NameSource::ALL.into_iter().find(|s| s.as_str() == raw)
    }

    /// The wire form. Written out rather than derived from `Debug`, because
    /// lowercasing `NoneDecorative` gives `nonedecorative`, which is not the
    /// value the contract publishes and is not a word.
    pub fn as_str(self) -> &'static str {
        match self {
            NameSource::Children => "children",
            NameSource::Label => "label",
            NameSource::AriaLabel => "aria_label",
            NameSource::AriaLabelledby => "aria_labelledby",
            NameSource::Title => "title",
            NameSource::Alt => "alt",
            NameSource::NoneDecorative => "none_decorative",
        }
    }
}

impl std::fmt::Display for NameSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct KeyboardContract {
    pub key: String,
    pub effect: String,
}

/// A contrast floor: a REQUIREMENT, never a realisation (VDS S-2(6)).
///
/// `boundary` and `against` name tokens or edges BY NAME. A literal value in
/// either is a storing-form violation of VDS S-2(2), and `no_stored_values`
/// looks for exactly that.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ContrastFloor {
    pub boundary: String,
    pub against: String,
    pub min_ratio: f64,
    /// Where the requirement comes from, e.g. "WCAG 2.2 SC 1.4.11".
    pub basis: String,
    #[serde(default)]
    pub scope: Option<FloorScope>,
}

/// What kind of thing the boundary is.
///
/// VDS S-9(5): where a lower floor is genuinely correct, the lawful move is to
/// change the component's SCOPE and state the basis, not to loosen the ratio. A
/// factual claim about scope is contestable by a reviewer; a quietly lowered
/// floor is not. That is why this field exists at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FloorScope {
    ControlBoundary,
    Text,
    GraphicalObject,
    Decoration,
}

impl FloorScope {
    pub fn as_str(self) -> &'static str {
        match self {
            FloorScope::ControlBoundary => "control_boundary",
            FloorScope::Text => "text",
            FloorScope::GraphicalObject => "graphical_object",
            FloorScope::Decoration => "decoration",
        }
    }
}

impl std::fmt::Display for FloorScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// How many routes consume this component. Measured, never estimated
/// (VDS S-5(7)), which is why the command that measured it is part of the value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Demand {
    pub routes: u32,
    pub measured_at: Timestamp,
    /// The exact command that produced the count.
    pub measured_by: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Amendment {
    pub at: Timestamp,
    pub by: String,
    pub kind: AmendmentKind,
    pub what: String,
    pub contract_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warrant_id: Option<WarrantId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proof_id: Option<ProofId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision_log_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AmendmentKind {
    NonBreaking,
    Breaking,
}

/// One reason an amendment is breaking under VDS S-9(4).
///
/// Classification is a pure function of the two records, so it is computed and
/// never declared: an author who says "non-breaking" about a removed prop is
/// contradicted by the records themselves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BreakingReason {
    pub what: String,
    /// True where the reason is specifically a LOWERED contrast floor, which
    /// VDS S-9(5) treats more strictly than other breaking changes.
    pub is_lowered_floor: bool,
}

impl std::fmt::Display for BreakingReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.what)
    }
}

/// Why `after` breaks the contract `before` published. VDS S-9(4).
///
/// The first word of that sentence is load-bearing: PUBLISHED. A record below
/// `registered` has published nothing. VDS.md S-5(4) puts the lifecycle on a
/// directed path and says `registered` is where "the contract is complete and
/// binding", so an edit to a `proposed` or `designed` record is the contract
/// being WRITTEN and cannot break a contract that does not yet exist.
///
/// Without that distinction the tool contradicted its own printed advice.
/// `vds register import` mints candidates at `proposed` and tells the author to
/// run `vds register amend --kind non_breaking ... --role button`, and setting a
/// role from null was classed breaking, and a breaking amendment demands a
/// warrant, and a warrant cannot be granted over a record nobody has registered.
/// A fresh candidate could therefore never receive the contract the same command
/// told the author to give it. Found by adopting VDS on `examples/storefront`,
/// which is the only way this class of defect is ever found: every unit test in
/// this file started from a `registered` record.
///
/// The guard is safe against being escaped. `set-status` only ever advances
/// along `Status::PATH`, so a record cannot be walked back to `proposed` to
/// launder a breaking change.
pub fn breaking_reasons(before: &ComponentRecord, after: &ComponentRecord) -> Vec<BreakingReason> {
    if !before.status.is_binding() {
        return Vec::new();
    }
    let mut reasons = Vec::new();
    let plain = |what: String| BreakingReason {
        what,
        is_lowered_floor: false,
    };

    let before_props: std::collections::BTreeMap<&str, &PropContract> =
        before.props.iter().map(|p| (p.name.as_str(), p)).collect();
    let after_props: std::collections::BTreeMap<&str, &PropContract> =
        after.props.iter().map(|p| (p.name.as_str(), p)).collect();

    for (name, _) in before_props
        .iter()
        .filter(|(n, _)| !after_props.contains_key(*n))
    {
        reasons.push(plain(format!("prop {name:?} removed")));
    }
    for (name, before_prop) in &before_props {
        if let Some(after_prop) = after_props.get(name) {
            if !before_prop.required && after_prop.required {
                reasons.push(plain(format!("prop {name:?} became required")));
            }
            if before_prop.type_expr != after_prop.type_expr {
                reasons.push(plain(format!(
                    "prop {name:?} type changed from {:?} to {:?}",
                    before_prop.type_expr, after_prop.type_expr
                )));
            }
        }
    }

    for state in State::ALL {
        if before.states.required.contains(&state) && !after.states.required.contains(&state) {
            reasons.push(plain(format!("required state {state:?} removed")));
        }
    }

    if before.a11y.role != after.a11y.role {
        reasons.push(plain(format!(
            "role changed from {:?} to {:?}",
            before.a11y.role, after.a11y.role
        )));
    }
    if before.a11y.accessible_name_source != after.a11y.accessible_name_source {
        reasons.push(plain(format!(
            "accessible-name source changed from {:?} to {:?}",
            before.a11y.accessible_name_source, after.a11y.accessible_name_source
        )));
    }

    // Keyboard. Absent from this list until it was noticed that `keyboard` could
    // not be amended at all, which hid the second half of the same hole: once
    // `--remove-keyboard` existed, a published keyboard contract could have been
    // withdrawn with no warrant and nothing saying so.
    //
    // Withdrawing a key or changing what it does is breaking for the same reason
    // removing a prop is: somebody is relying on it, and a keyboard contract is
    // relied on by a person who cannot use a mouse. ADDING a key is not breaking,
    // which is the same asymmetry the prop rules already use.
    let before_keys: std::collections::BTreeMap<&str, &str> = before
        .a11y
        .keyboard
        .iter()
        .map(|k| (k.key.as_str(), k.effect.as_str()))
        .collect();
    let after_keys: std::collections::BTreeMap<&str, &str> = after
        .a11y
        .keyboard
        .iter()
        .map(|k| (k.key.as_str(), k.effect.as_str()))
        .collect();
    for (key, before_effect) in &before_keys {
        match after_keys.get(key) {
            None => reasons.push(plain(format!("keyboard contract {key:?} removed"))),
            Some(after_effect) if after_effect != before_effect => {
                reasons.push(plain(format!(
                    "keyboard contract {key:?} changed from {before_effect:?} to \
                     {after_effect:?}"
                )));
            }
            Some(_) => {}
        }
    }

    let key = |f: &ContrastFloor| (f.boundary.clone(), f.against.clone());
    let before_floors: std::collections::BTreeMap<(String, String), f64> = before
        .a11y
        .contrast_floors
        .iter()
        .map(|f| (key(f), f.min_ratio))
        .collect();
    let after_floors: std::collections::BTreeMap<(String, String), f64> = after
        .a11y
        .contrast_floors
        .iter()
        .map(|f| (key(f), f.min_ratio))
        .collect();

    for ((boundary, against), before_ratio) in &before_floors {
        match after_floors.get(&(boundary.clone(), against.clone())) {
            None => reasons.push(plain(format!(
                "contrast floor {boundary} against {against} removed"
            ))),
            Some(after_ratio) if after_ratio < before_ratio => reasons.push(BreakingReason {
                what: format!(
                    "contrast floor {boundary} against {against} LOWERED from {before_ratio} \
                     to {after_ratio}"
                ),
                is_lowered_floor: true,
            }),
            Some(_) => {}
        }
    }

    reasons
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record() -> ComponentRecord {
        ComponentRecord {
            id: ComponentId::parse("CMP-0001").unwrap(),
            name: "Button".into(),
            status: Status::Registered,
            contract_version: 1,
            figma: None,
            code: Some(CodeCounterpart {
                import_path: "@/components/ui".into(),
                source_file: "src/components/ui/button.tsx".into(),
                export_name: "Button".into(),
            }),
            props: vec![PropContract {
                name: "variant".into(),
                type_expr: "primary|ghost".into(),
                required: false,
                figma_property: None,
            }],
            states: StateContract {
                required: vec![State::Default, State::Focus],
                drawn: vec![State::Default],
                built: vec![],
            },
            a11y: Accessibility {
                role: Some("button".into()),
                accessible_name_source: NameSource::Children,
                keyboard: vec![],
                contrast_floors: vec![ContrastFloor {
                    boundary: "control-border".into(),
                    against: "surface".into(),
                    min_ratio: 3.0,
                    basis: "WCAG 2.2 SC 1.4.11".into(),
                    scope: Some(FloorScope::ControlBoundary),
                }],
            },
            demand: Demand {
                routes: 0,
                measured_at: Timestamp::fixed(2026, 7, 25, 10, 0, 0),
                measured_by: "vds register measure-demand".into(),
            },
            supersedes: vec![],
            superseded_by: None,
            amendments: vec![],
            basis: vec!["ACT-VDS-001:s5".into()],
            deprecated_at: None,
            retired_at: None,
            retirement_proof_id: None,
            notes: None,
        }
    }

    #[test]
    fn required_not_drawn_reports_in_specification_order() {
        assert_eq!(record().required_not_drawn(), vec![State::Focus]);
    }

    #[test]
    fn an_unchanged_record_breaks_nothing() {
        assert!(breaking_reasons(&record(), &record()).is_empty());
    }

    #[test]
    fn removing_a_prop_is_breaking() {
        let mut after = record();
        after.props.clear();
        let reasons = breaking_reasons(&record(), &after);
        assert_eq!(reasons.len(), 1, "{reasons:?}");
        assert!(reasons[0].what.contains("removed"));
    }

    #[test]
    fn tightening_a_floor_is_not_breaking_and_lowering_one_is() {
        let mut tightened = record();
        tightened.a11y.contrast_floors[0].min_ratio = 4.5;
        assert!(
            breaking_reasons(&record(), &tightened).is_empty(),
            "a floor may be tightened by any project (VDS S-9(5))"
        );

        let mut lowered = record();
        lowered.a11y.contrast_floors[0].min_ratio = 1.2;
        let reasons = breaking_reasons(&record(), &lowered);
        assert_eq!(reasons.len(), 1);
        assert!(reasons[0].is_lowered_floor, "{reasons:?}");
        assert!(reasons[0].what.contains("LOWERED"));
    }

    #[test]
    fn removing_a_floor_is_breaking_but_is_not_a_lowered_floor() {
        let mut after = record();
        after.a11y.contrast_floors.clear();
        let reasons = breaking_reasons(&record(), &after);
        assert_eq!(reasons.len(), 1);
        assert!(!reasons[0].is_lowered_floor);
    }

    #[test]
    fn changing_a_prop_type_is_breaking() {
        let mut after = record();
        after.props[0].type_expr = "string".into();
        assert_eq!(breaking_reasons(&record(), &after).len(), 1);
    }

    #[test]
    fn removing_a_required_state_is_breaking() {
        let mut after = record();
        after.states.required = vec![State::Default];
        assert_eq!(breaking_reasons(&record(), &after).len(), 1);
    }

    #[test]
    fn adding_an_optional_prop_is_not_breaking() {
        let mut after = record();
        after.props.push(PropContract {
            name: "size".into(),
            type_expr: "sm|md".into(),
            required: false,
            figma_property: None,
        });
        assert!(breaking_reasons(&record(), &after).is_empty());
    }

    #[test]
    fn state_buckets_normalise_into_specification_order() {
        let contract = StateContract {
            required: vec![State::Focus, State::Default, State::Focus],
            drawn: vec![],
            built: vec![],
        }
        .normalised();
        assert_eq!(contract.required, vec![State::Default, State::Focus]);
    }

    #[test]
    fn an_unknown_field_is_refused_rather_than_ignored() {
        let mut value = serde_json::to_value(record()).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert("colour".into(), serde_json::json!("#ebebeb"));
        assert!(
            serde_json::from_value::<ComponentRecord>(value).is_err(),
            "an unknown field is where a realisation would smuggle itself in"
        );
    }

    #[test]
    fn every_enum_as_str_matches_its_wire_form() {
        for source in [
            NameSource::Children,
            NameSource::AriaLabel,
            NameSource::AriaLabelledby,
            NameSource::Title,
            NameSource::Alt,
            NameSource::NoneDecorative,
        ] {
            assert_eq!(
                serde_json::to_string(&source).unwrap(),
                format!("\"{}\"", source.as_str()),
                "as_str and the wire form must not drift"
            );
        }
        for scope in [
            FloorScope::ControlBoundary,
            FloorScope::Text,
            FloorScope::GraphicalObject,
            FloorScope::Decoration,
        ] {
            assert_eq!(
                serde_json::to_string(&scope).unwrap(),
                format!("\"{}\"", scope.as_str())
            );
        }
    }

    #[test]
    fn the_record_round_trips_through_yaml() {
        let text = serde_yaml::to_string(&record()).unwrap();
        assert_eq!(
            serde_yaml::from_str::<ComponentRecord>(&text).unwrap(),
            record()
        );
    }

    /// The defect adoption found: a fresh candidate could never be given its
    /// contract.
    ///
    /// `vds register import` mints at `proposed` and prints advice telling the
    /// author to set the role with a `non_breaking` amendment. Setting a role
    /// from null was classed breaking, a breaking amendment demands a warrant,
    /// and no warrant can exist over a record nobody has registered. Every unit
    /// test in this file started from a `registered` record, so none of them saw
    /// it.
    #[test]
    fn writing_the_contract_on_an_unregistered_record_breaks_nothing() {
        for status in [Status::Proposed, Status::Designed] {
            let mut before = record();
            before.status = status;
            before.a11y.role = None;
            before.a11y.contrast_floors = vec![];
            before.states.required = vec![];

            let mut after = before.clone();
            after.a11y.role = Some("button".into());
            after.a11y.accessible_name_source = NameSource::Label;
            after.states.required = vec![State::Focus];

            assert!(
                breaking_reasons(&before, &after).is_empty(),
                "{status} published no contract, so there was nothing to break: {:?}",
                breaking_reasons(&before, &after)
            );
        }
    }

    /// The other direction, and the one that keeps the guard honest. The exact
    /// same diff against a BINDING record is breaking, and stays breaking at
    /// every status from `registered` onward.
    #[test]
    fn the_same_change_to_a_binding_record_is_still_breaking() {
        for status in [
            Status::Registered,
            Status::Built,
            Status::Verified,
            Status::Deprecated,
            Status::Retired,
        ] {
            let mut before = record();
            before.status = status;
            before.a11y.role = None;
            let mut after = before.clone();
            after.a11y.role = Some("button".into());

            assert!(
                !breaking_reasons(&before, &after).is_empty(),
                "a role change at {status} stopped being breaking, so a published contract \
                 can be rewritten without a warrant"
            );
        }
    }

    /// A lowered floor is the change VDS S-9(5) cares most about, so check the
    /// guard did not open a route to it.
    #[test]
    fn a_lowered_floor_on_a_binding_record_is_still_flagged_as_lowered() {
        let mut before = record();
        before.status = Status::Registered;
        before.a11y.contrast_floors = vec![ContrastFloor {
            boundary: "control-boundary".into(),
            against: "surface".into(),
            min_ratio: 4.5,
            basis: "WCAG 2.2 SC 1.4.3".into(),
            scope: None,
        }];
        let mut after = before.clone();
        after.a11y.contrast_floors[0].min_ratio = 3.0;

        let reasons = breaking_reasons(&before, &after);
        assert_eq!(reasons.len(), 1, "{reasons:?}");
        assert!(reasons[0].is_lowered_floor);
    }

    /// Every variant round-trips through the wire form and back, so `ALL` and
    /// `as_str` cannot disagree and the CLI's derived parse cannot refuse a
    /// value the record can hold.
    #[test]
    fn every_accessible_name_source_parses_from_its_own_wire_form() {
        for source in NameSource::ALL {
            assert_eq!(NameSource::parse(source.as_str()), Some(source));
        }
        assert_eq!(NameSource::parse("label"), Some(NameSource::Label));
        assert_eq!(NameSource::parse("Label"), None, "the wire form is exact");
        assert_eq!(NameSource::parse("nonedecorative"), None);

        let mut seen: Vec<&str> = NameSource::ALL.iter().map(|s| s.as_str()).collect();
        let count = seen.len();
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), count, "two variants share a wire form");
    }
}
