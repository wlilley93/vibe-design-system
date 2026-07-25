//! The **generation brief**: what an agent generating into Figma is allowed to
//! draw.
//!
//! This is the prompt-to-Figma half of the loop, and it exists because of the
//! second founding defect at VDS S-1(4)(b): "the declared showpiece screen family
//! was drawn entirely in the outgoing card idiom while the doctrine requiring
//! flush panels on a hairline existed only in prose, and nothing read that
//! prose."
//!
//! **Nothing read that prose.** A brief is that prose turned into something a
//! generator reads. Handing an agent the register before it draws is the only
//! intervention that acts at authoring time; every check after that is an audit,
//! and VDS S-1(5) is explicit that converting a late audit into an authoring-time
//! failure is the whole return.
//!
//! # What a brief may and may not contain
//!
//! A brief holds REQUIREMENTS and never REALISATIONS (VDS S-2(4)). It names the
//! components that exist, the states each must draw, the props each declares,
//! the roles and keyboard contracts, and the contrast floors that bind. It
//! contains no colour, no length, no font, no duration and no easing curve,
//! because those live in the Figma file and in `app/globals.css`, and a brief
//! that carried them would make VDS the fourth authority [2026] VJS-CC-OPBOX 3
//! forbids.
//!
//! That is not a limitation working around a rule. It is the correct division:
//! the brief says WHAT MUST EXIST and the design system says WHAT IT LOOKS LIKE.
//! An agent handed both draws inside the contract; an agent handed only the
//! second draws whatever it likes and someone finds out months later.

use serde::{Deserialize, Serialize};
use vds_core::{
    ComponentId, ComponentRecord, Digest, Result, State, Status, Timestamp, Warrant, WarrantStatus,
};
use vds_store::Store;

use crate::ledger::FigmaLedger;

pub const BRIEF_SCHEMA_VERSION: u32 = 1;

/// One component an agent may draw, and the contract it must draw inside.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BriefComponent {
    pub id: ComponentId,
    pub name: String,
    pub status: Status,
    /// The Figma node to edit, where the register names one. Absent means the
    /// component has never been drawn and this brief is asking for it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub figma_node: Option<String>,
    /// Every state this component MUST draw.
    pub states_required: Vec<State>,
    /// The states already drawn, MEASURED from the decided-target file where a
    /// Figma ledger is present, and taken from the record where it is not. Which
    /// of the two it is, is stated on the brief rather than left to be assumed.
    pub states_drawn: Vec<State>,
    /// Required and not yet drawn. This is the work the brief is asking for.
    pub states_to_draw: Vec<State>,
    /// The prop contract, as `name: type (required|optional)`.
    pub props: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    pub accessible_name_source: String,
    pub keyboard: Vec<String>,
    /// Contrast floors, as requirements with their basis. A floor is a duty
    /// drawn from an external standard, never a colour.
    pub contrast_floors: Vec<String>,
    /// How many routes consume it, and when that was measured.
    pub demand_routes: u32,
}

/// The brief itself.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenerationBrief {
    pub schema_version: u32,
    pub generated_at: Timestamp,
    pub generated_by: String,
    pub project: String,
    /// The surface this brief is bounded by. Every claim in it is true of this
    /// register and no other, and the digest says which.
    pub register_digest: Digest,
    /// Where `states_drawn` came from.
    pub states_drawn_measured_from: DrawnSource,
    /// The components an agent may compose with.
    pub may_use: Vec<BriefComponent>,
    /// Components that exist and may NOT be used, with the reason.
    pub may_not_use: Vec<Forbidden>,
    /// The rules that bind the generation, in the imperative.
    pub rules: Vec<String>,
    /// What this brief does not settle, so an agent does not read silence as
    /// permission.
    pub not_settled: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DrawnSource {
    /// Measured from the decided-target Figma file. The register's own `drawn`
    /// list was not consulted.
    FigmaLedger,
    /// Taken from the register's hand-maintained `drawn` list, because no Figma
    /// ledger is present. VDS S-5(5): a hand-maintained register decays.
    RegisterRecord,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Forbidden {
    pub id: ComponentId,
    pub name: String,
    pub reason: String,
    /// Where the component was deprecated toward a successor, the successor to
    /// use instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_instead: Option<ComponentId>,
}

/// Build the brief from the register, and from the Figma ledger where one is
/// present.
pub fn build(
    store: &Store,
    figma: Option<&FigmaLedger>,
    w1: Option<&Warrant>,
) -> Result<GenerationBrief> {
    let records = store.read_register()?;
    let mut may_use = Vec::new();
    let mut may_not_use = Vec::new();

    for located in &records {
        let record = &located.value;
        match record.status {
            Status::Retired => {
                may_not_use.push(Forbidden {
                    id: record.id.clone(),
                    name: record.name.clone(),
                    reason: format!(
                        "retired at {}. VDS S-9(8) inverts the test after retirement: drawing \
                         with it is the defect.",
                        record
                            .retired_at
                            .as_ref()
                            .map(|t| t.to_string())
                            .unwrap_or_else(|| "an unrecorded time".into())
                    ),
                    use_instead: record.superseded_by.clone(),
                });
                continue;
            }
            Status::Deprecated => {
                may_not_use.push(Forbidden {
                    id: record.id.clone(),
                    name: record.name.clone(),
                    reason: match &record.superseded_by {
                        Some(successor) => format!("deprecated, superseded by {successor}"),
                        None => "deprecated and withdrawn outright, with no replacement".into(),
                    },
                    use_instead: record.superseded_by.clone(),
                });
                continue;
            }
            Status::Proposed => {
                may_not_use.push(Forbidden {
                    id: record.id.clone(),
                    name: record.name.clone(),
                    reason: "proposed but not registered. VDS S-6(2): a component must be \
                             registered before anything composes with it, and composing with a \
                             merely proposed one is exactly the drift the anti-drift proof \
                             exists to catch."
                        .into(),
                    use_instead: None,
                });
                continue;
            }
            _ => {}
        }
        may_use.push(component(record, figma));
    }

    let source = if figma.is_some() {
        DrawnSource::FigmaLedger
    } else {
        DrawnSource::RegisterRecord
    };

    let mut not_settled = vec![
        "VDS S-9(10) RESERVED (SUBMISSION-VDS-005): where the primitive floor sits is \
         unsettled. Bare HTML elements are informational rows in every proof, so this brief \
         does not reach the primitive layer, and a screen built entirely from bare elements \
         satisfies it while proving nothing."
            .to_owned(),
    ];
    if figma.is_none() {
        not_settled.push(
            "No Figma ledger is present, so `states_drawn` is the register's own \
             hand-maintained claim rather than a measurement of the decided-target file. \
             VDS S-5(5): a hand-maintained register decays. Run `vds figma pull` to measure it."
                .to_owned(),
        );
    }

    let mut rules = vec![
        "Compose only from `may_use`. A component that is not in the register does not exist \
         for the purposes of this brief, and drawing one is drift (VDS S-7(5) composition)."
            .to_owned(),
        "Draw every state in `states_to_draw`. A required state that is not drawn fails the \
         `states` proof and blocks W2 (VDS S-6(2))."
            .to_owned(),
        "Do not invent a tenth state. The nine are fixed by VDS S-5(3): default, hover, focus, \
         active, selected, disabled, loading, error, success."
            .to_owned(),
        "Do not change a component's prop contract while drawing. A contract change is an \
         amendment with its own record and, where it is breaking, its own warrant \
         (VDS S-9(2), S-9(4))."
            .to_owned(),
        "Respect every contrast floor. They are requirements drawn from WCAG, not preferences, \
         and a floor may be tightened and never loosened (VDS S-9(5))."
            .to_owned(),
        "Where a component you need does not exist, STOP and register it first. Registering \
         after drawing is the ordering VDS S-6(2) forbids, and it is the ordering under which \
         every drift defect in the motivating project was authored."
            .to_owned(),
    ];

    match w1 {
        Some(warrant) if warrant.status == WarrantStatus::Granted => rules.push(format!(
            "Design may begin: {} is granted over this surface (W1 REGISTER-COMPLETE).",
            warrant.id
        )),
        _ => rules.push(
            "W1 REGISTER-COMPLETE is NOT granted over this surface. VDS S-6(2) makes W1 the \
             gate that unlocks design, so work done now is done ahead of its warrant and will \
             have to be re-justified. This brief tells you what the register currently says; \
             it does not authorise the work."
                .to_owned(),
        ),
    }

    Ok(GenerationBrief {
        schema_version: BRIEF_SCHEMA_VERSION,
        generated_at: Timestamp::now(),
        generated_by: "vds brief".to_owned(),
        project: store.project.config.jurisdiction_id.clone(),
        register_digest: store.register_digest()?,
        states_drawn_measured_from: source,
        may_use,
        may_not_use,
        rules,
        not_settled,
    })
}

fn component(record: &ComponentRecord, figma: Option<&FigmaLedger>) -> BriefComponent {
    let measured = figma
        .and_then(|ledger| ledger.row(&record.id))
        .filter(|row| row.resolved)
        .map(|row| row.states_drawn.clone());
    let drawn = measured.unwrap_or_else(|| record.states.drawn.clone());
    let to_draw: Vec<State> = State::ALL
        .into_iter()
        .filter(|s| record.states.required.contains(s) && !drawn.contains(s))
        .collect();

    BriefComponent {
        id: record.id.clone(),
        name: record.name.clone(),
        status: record.status,
        figma_node: record.figma.as_ref().map(|f| f.node_id.clone()),
        states_required: record.states.required.clone(),
        states_drawn: drawn,
        states_to_draw: to_draw,
        props: record
            .props
            .iter()
            .map(|p| {
                format!(
                    "{}: {} ({})",
                    p.name,
                    p.type_expr,
                    if p.required { "required" } else { "optional" }
                )
            })
            .collect(),
        role: record.a11y.role.clone(),
        accessible_name_source: record.a11y.accessible_name_source.as_str().to_owned(),
        keyboard: record
            .a11y
            .keyboard
            .iter()
            .map(|k| format!("{} {}", k.key, k.effect))
            .collect(),
        contrast_floors: record
            .a11y
            .contrast_floors
            .iter()
            .map(|f| {
                format!(
                    "{} against {} at least {}:1 ({})",
                    f.boundary, f.against, f.min_ratio, f.basis
                )
            })
            .collect(),
        demand_routes: record.demand.routes,
    }
}

impl GenerationBrief {
    /// The brief as prose, for pasting into a prompt.
    ///
    /// Markdown rather than YAML because the consumer is a generating agent, and
    /// an agent given a schema dump reads it as data to be echoed rather than as
    /// instructions to be followed.
    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("# Design brief: {}\n\n", self.project));
        out.push_str(
            "This brief is derived from the component register. It is the contract the design \
             must be drawn inside. It contains no colours, lengths, fonts, durations or easing \
             curves: those live in the design system itself, and this brief does not overrule \
             them.\n\n",
        );
        out.push_str(&format!("Register digest: `{}`\n", self.register_digest));
        out.push_str(&format!(
            "States drawn measured from: **{}**\n\n",
            match self.states_drawn_measured_from {
                DrawnSource::FigmaLedger => "the decided-target Figma file",
                DrawnSource::RegisterRecord =>
                    "the register's own claim (NOT measured; run `vds figma pull`)",
            }
        ));

        out.push_str("## Rules\n\n");
        for rule in &self.rules {
            out.push_str(&format!("- {rule}\n"));
        }

        out.push_str(&format!("\n## Components you may use ({})\n\n", self.may_use.len()));
        for component in &self.may_use {
            out.push_str(&format!(
                "### {} `{}` ({})\n\n",
                component.name, component.id, component.status
            ));
            if let Some(node) = &component.figma_node {
                out.push_str(&format!("- Figma node: `{node}`\n"));
            } else {
                out.push_str("- Figma node: **none recorded**. This component has never been drawn.\n");
            }
            out.push_str(&format!(
                "- States required: {}\n",
                join_states(&component.states_required)
            ));
            out.push_str(&format!(
                "- States drawn: {}\n",
                join_states(&component.states_drawn)
            ));
            if component.states_to_draw.is_empty() {
                out.push_str("- **Nothing to draw**: every required state is drawn.\n");
            } else {
                out.push_str(&format!(
                    "- **STILL TO DRAW: {}**\n",
                    join_states(&component.states_to_draw)
                ));
            }
            if !component.props.is_empty() {
                out.push_str(&format!("- Props: {}\n", component.props.join("; ")));
            }
            if let Some(role) = &component.role {
                out.push_str(&format!("- Role: `{role}`\n"));
            }
            out.push_str(&format!(
                "- Accessible name from: {}\n",
                component.accessible_name_source
            ));
            if !component.keyboard.is_empty() {
                out.push_str(&format!("- Keyboard: {}\n", component.keyboard.join("; ")));
            }
            for floor in &component.contrast_floors {
                out.push_str(&format!("- Contrast floor: {floor}\n"));
            }
            out.push_str(&format!(
            "- Consumed by {} {}\n\n",
            component.demand_routes,
            if component.demand_routes == 1 { "route" } else { "routes" }
        ));
        }

        if !self.may_not_use.is_empty() {
            out.push_str(&format!(
                "## Components you may NOT use ({})\n\n",
                self.may_not_use.len()
            ));
            for forbidden in &self.may_not_use {
                out.push_str(&format!(
                    "- **{}** `{}`: {}",
                    forbidden.name, forbidden.id, forbidden.reason
                ));
                if let Some(successor) = &forbidden.use_instead {
                    out.push_str(&format!(" Use `{successor}` instead."));
                }
                out.push('\n');
            }
            out.push('\n');
        }

        out.push_str("## What this brief does not settle\n\n");
        for item in &self.not_settled {
            out.push_str(&format!("- {item}\n"));
        }
        out
    }
}

fn join_states(states: &[State]) -> String {
    if states.is_empty() {
        return "none".to_owned();
    }
    states
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Fixture;

    #[test]
    fn a_registered_component_may_be_used_and_a_proposed_one_may_not() {
        let f = Fixture::new();
        f.register("Button", Status::Registered);
        f.register("Sketch", Status::Proposed);
        let store = f.store();
        let brief = build(&store, None, None).unwrap();
        assert_eq!(brief.may_use.len(), 1);
        assert_eq!(brief.may_use[0].name, "Button");
        assert_eq!(brief.may_not_use.len(), 1);
        assert!(brief.may_not_use[0].reason.contains("registered"));
    }

    #[test]
    fn a_deprecated_component_names_its_successor() {
        let f = Fixture::new();
        let successor = f.register("NewCard", Status::Registered);
        let old = f.register("OldCard", Status::Registered);
        f.amend(&old, |r| {
            r.status = Status::Deprecated;
            r.superseded_by = Some(successor.clone());
        });
        let store = f.store();
        let brief = build(&store, None, None).unwrap();
        let forbidden = brief
            .may_not_use
            .iter()
            .find(|x| x.name == "OldCard")
            .unwrap();
        assert_eq!(forbidden.use_instead, Some(successor));
    }

    #[test]
    fn states_to_draw_is_the_work_the_brief_is_asking_for() {
        let f = Fixture::new();
        let id = f.register("Button", Status::Registered);
        f.amend(&id, |r| {
            r.states.required = vec![State::Default, State::Hover, State::Focus];
            r.states.drawn = vec![State::Default];
        });
        let store = f.store();
        let brief = build(&store, None, None).unwrap();
        assert_eq!(
            brief.may_use[0].states_to_draw,
            vec![State::Hover, State::Focus]
        );
    }

    #[test]
    fn the_brief_says_where_states_drawn_came_from() {
        let f = Fixture::new();
        f.register("Button", Status::Registered);
        let store = f.store();
        let brief = build(&store, None, None).unwrap();
        assert_eq!(brief.states_drawn_measured_from, DrawnSource::RegisterRecord);
        assert!(
            brief
                .not_settled
                .iter()
                .any(|n| n.contains("hand-maintained")),
            "a brief resting on an unmeasured claim must say so: {:?}",
            brief.not_settled
        );
        assert!(brief.to_markdown().contains("NOT measured"));
    }

    #[test]
    fn a_figma_ledger_overrides_the_registers_own_claim_about_what_is_drawn() {
        let f = Fixture::new();
        let id = f.register("Button", Status::Registered);
        f.amend(&id, |r| {
            r.states.required = vec![State::Default, State::Hover];
            // The register CLAIMS both are drawn.
            r.states.drawn = vec![State::Default, State::Hover];
        });
        // The decided-target file draws only one of them.
        let ledger = crate::testing::figma_ledger(&[(&id, vec![State::Default])]);
        let store = f.store();
        let brief = build(&store, Some(&ledger), None).unwrap();
        assert_eq!(brief.states_drawn_measured_from, DrawnSource::FigmaLedger);
        assert_eq!(
            brief.may_use[0].states_to_draw,
            vec![State::Hover],
            "the file that decides is the file that answers, not the record that claims"
        );
    }

    #[test]
    fn a_brief_says_when_design_is_not_yet_warranted() {
        let f = Fixture::new();
        f.register("Button", Status::Registered);
        let store = f.store();
        let brief = build(&store, None, None).unwrap();
        assert!(
            brief.rules.iter().any(|r| r.contains("NOT granted")),
            "{:?}",
            brief.rules
        );
    }

    /// VDS S-2(4). The brief is handed to a generator, so a realisation in it
    /// would be a design value VDS handed down, which makes VDS an authority.
    #[test]
    fn a_brief_carries_no_realisation() {
        let f = Fixture::new();
        f.register("Button", Status::Registered);
        let store = f.store();
        let brief = build(&store, None, None).unwrap();
        let text = brief.to_markdown() + &serde_yaml::to_string(&brief).unwrap();
        if let Some(found) = crate::testing::realisation_in(&text) {
            panic!("the brief contains {found:?}, which is a realisation (VDS S-2(4))");
        }
    }

    #[test]
    fn the_markdown_names_every_component_and_every_rule() {
        let f = Fixture::new();
        f.register("Button", Status::Registered);
        let store = f.store();
        let brief = build(&store, None, None).unwrap();
        let markdown = brief.to_markdown();
        assert!(markdown.contains("Button"));
        assert!(markdown.contains("CMP-0001"));
        for rule in &brief.rules {
            let head: String = rule.chars().take(30).collect();
            assert!(markdown.contains(&head), "rule missing from markdown: {rule}");
        }
    }
}
