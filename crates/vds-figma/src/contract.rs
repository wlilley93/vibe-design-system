//! The **implementation contract**: what a Figma node must become in code.
//!
//! This is the Figma-to-code half of the loop. The brief governs what gets
//! drawn; this governs what the drawing becomes.
//!
//! The design point is small and it is the whole thing: **the agent writing the
//! code is handed the criteria it will be judged against, before it writes.**
//! Every check in VDS is otherwise an audit that happens afterwards, and an
//! audit that happens afterwards produces a defect plus a re-work, where the
//! same rule stated beforehand produces neither. VDS S-1(5) says the return on
//! VDS is converting a late finding into an authoring-time one; a contract is
//! that, one step earlier than a proof.
//!
//! Like the brief, a contract holds REQUIREMENTS and never REALISATIONS
//! (VDS S-2(4)). It says the component must expose a `variant` prop of a named
//! type, must draw a focus state, must clear 3.0:1 at its control boundary, and
//! must be reachable by Enter. It does not say what focus LOOKS like. That is in
//! the Figma node the contract points at, which is the system of record for what
//! is decided, and reading it is the implementer's job rather than VDS's.

use serde::{Deserialize, Serialize};
use vds_core::{ComponentId, Digest, ProofKind, Result, State, Status, Timestamp, VdsError};
use vds_store::Store;

use crate::ledger::FigmaLedger;

pub const CONTRACT_SCHEMA_VERSION: u32 = 1;

/// What must be true of the code before this component is `built`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImplementationContract {
    pub schema_version: u32,
    pub generated_at: Timestamp,
    pub generated_by: String,
    pub component: ComponentId,
    pub name: String,
    pub status: Status,
    /// The digest of the record this contract was derived from. A contract and
    /// a record that disagree are two contracts, and this says which one this is.
    pub record_digest: Digest,

    /// Where to read what it must LOOK like. VDS holds none of that.
    pub read_the_design_from: DesignSource,
    /// Where the code goes.
    pub write_the_code_to: Option<CodeTarget>,

    /// Every requirement, as an imperative a reader can check off.
    pub must: Vec<Requirement>,
    /// What no check will catch, so an implementer does not read a passing gate
    /// as a passing design.
    pub not_checked: Vec<String>,
    /// The proofs that will be run against this work, named so the implementer
    /// can run them first.
    pub judged_by: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignSource {
    pub authority_for: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub figma_file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub figma_node: Option<String>,
    /// Whether that node was measured to resolve, and when. Absent where no
    /// Figma ledger has been pulled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved: Option<bool>,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeTarget {
    pub source_file: String,
    pub import_path: String,
    pub export_name: String,
    pub exists: bool,
}

/// One requirement, with the clause that imposes it and the proof that will
/// check it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Requirement {
    pub what: String,
    pub basis: String,
    /// The proof kind that checks it, or `None` where nothing does. A
    /// requirement nothing checks is still a requirement, and saying which ones
    /// those are is more honest than listing only the checkable ones.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checked_by: Option<String>,
}

pub fn build(
    store: &Store,
    id: &ComponentId,
    figma: Option<&FigmaLedger>,
) -> Result<ImplementationContract> {
    let located = store.read_record(id)?;
    let record = &located.value;

    if record.status == Status::Retired {
        return Err(VdsError::precondition(format!(
            "{id} is retired. VDS S-9(8) inverts the test after retirement: implementing it \
             would make the code the defect. There is no lawful contract to issue."
        )));
    }

    let figma_row = figma.and_then(|l| l.row(id));
    let read_from = DesignSource {
        authority_for: "what is decided".to_owned(),
        figma_file: record.figma.as_ref().map(|f| f.file_key.clone()),
        figma_node: record.figma.as_ref().map(|f| f.node_id.clone()),
        resolved: figma_row.map(|r| r.resolved),
        note: match (&record.figma, figma_row) {
            (None, _) => "The register names no Figma node for this component, so there is \
                          nothing decided to implement against. Draw it first, or record the \
                          node with `vds register amend --figma`."
                .to_owned(),
            (Some(_), Some(row)) if !row.resolved => format!(
                "The register names a node that did NOT resolve in the pinned file: {}. \
                 Implementing against a node that is not there means implementing against \
                 nothing.",
                row.unresolved_because
                    .clone()
                    .unwrap_or_else(|| "no reason recorded".into())
            ),
            (Some(_), Some(_)) => "Read every colour, length, radius, font, duration and easing \
                                   curve from this node. VDS holds none of them, by design: \
                                   [2026] VJS-CC-OPBOX 3 D1 makes the Figma file the system of \
                                   record for what is decided, and VDS reads it and never \
                                   overrules it."
                .to_owned(),
            (Some(_), None) => "No Figma ledger has been pulled, so nothing has confirmed this \
                                node still exists. Run `vds figma pull` before relying on it."
                .to_owned(),
        },
    };

    let write_to = record.code.as_ref().map(|code| CodeTarget {
        source_file: code.source_file.clone(),
        import_path: code.import_path.clone(),
        export_name: code.export_name.clone(),
        exists: store.project.root.join(&code.source_file).is_file(),
    });

    let mut must = Vec::new();

    if let Some(target) = &write_to {
        must.push(Requirement {
            what: format!(
                "export `{}` from `{}`, importable as `{}`",
                target.export_name, target.source_file, target.import_path
            ),
            basis: "the register's code counterpart".to_owned(),
            checked_by: Some(ProofKind::Reconciliation.as_str().to_owned()),
        });
    } else {
        must.push(Requirement {
            what: "record a code counterpart before reaching `built`: an import path, a source \
                   file and an export name"
                .to_owned(),
            basis: "VDS S-5(2): the record must be comparable with the code".to_owned(),
            checked_by: Some(ProofKind::Reconciliation.as_str().to_owned()),
        });
    }

    for prop in &record.props {
        must.push(Requirement {
            what: format!(
                "accept a prop `{}` of type `{}`, {}",
                prop.name,
                prop.type_expr,
                if prop.required {
                    "required"
                } else {
                    "optional"
                }
            ),
            basis: format!(
                "the prop contract at contractVersion {}",
                record.contract_version
            ),
            checked_by: Some(ProofKind::Parity.as_str().to_owned()),
        });
    }

    let drawn = figma_row
        .filter(|r| r.resolved)
        .map(|r| r.states_drawn.clone())
        .unwrap_or_else(|| record.states.drawn.clone());
    for state in State::ALL {
        if !record.states.required.contains(&state) {
            continue;
        }
        let is_drawn = drawn.contains(&state);
        must.push(Requirement {
            what: format!(
                "implement the `{state}` state{}",
                if is_drawn {
                    ""
                } else {
                    ", which is NOT yet drawn in the decided-target file, so it must be \
                     designed before it can be implemented faithfully"
                }
            ),
            basis: "VDS S-5(3): the nine states are fixed and this one is required".to_owned(),
            checked_by: Some(ProofKind::Parity.as_str().to_owned()),
        });
    }

    if let Some(role) = &record.a11y.role {
        must.push(Requirement {
            what: format!("expose the ARIA role `{role}`"),
            basis: "the accessibility contract".to_owned(),
            checked_by: Some(ProofKind::Parity.as_str().to_owned()),
        });
    }
    must.push(Requirement {
        what: format!(
            "take its accessible name from {}",
            record.a11y.accessible_name_source
        ),
        basis: "the accessibility contract".to_owned(),
        checked_by: Some(ProofKind::Parity.as_str().to_owned()),
    });
    for key in &record.a11y.keyboard {
        must.push(Requirement {
            what: format!("respond to `{}` by: {}", key.key, key.effect),
            basis: "the keyboard contract".to_owned(),
            checked_by: None,
        });
    }
    for floor in &record.a11y.contrast_floors {
        must.push(Requirement {
            what: format!(
                "keep `{}` against `{}` at {}:1 or better{}",
                floor.boundary,
                floor.against,
                floor.min_ratio,
                match floor.scope {
                    Some(scope) => format!(" (scope: {scope})"),
                    None => String::new(),
                }
            ),
            basis: floor.basis.clone(),
            checked_by: Some(ProofKind::Contrast.as_str().to_owned()),
        });
    }

    let mut not_checked = vec![
        "Whether the result looks right. VDS S-1(6): VDS checks contracts, floors, composition \
         and parity, and whether a surface is good is reserved to the Principal at W3. No \
         accumulation of passing proofs substitutes for that."
            .to_owned(),
    ];
    for kind in [ProofKind::Parity, ProofKind::Contrast, ProofKind::TokenPin] {
        if let Some(why) = kind.unimplemented_because() {
            not_checked.push(format!(
                "The `{kind}` proof is specified and NOT implemented in this build, so the \
                 requirements above marked `checked_by: {kind}` are currently checked by \
                 nothing. Why it is unimplemented: {why}"
            ));
        }
    }
    if figma.is_none() {
        not_checked.push(
            "No Figma ledger is present, so nothing has confirmed the node this contract points \
             at still exists, and `states_drawn` is the register's own claim rather than a \
             measurement (VDS S-5(5))."
                .to_owned(),
        );
    }

    let judged_by = vec![
        format!("vds proof {}", ProofKind::Reconciliation),
        format!("vds proof {}", ProofKind::Composition),
        format!("vds proof {}", ProofKind::States),
        format!(
            "vds proof {}  (not implemented in this build)",
            ProofKind::Parity
        ),
        format!(
            "vds proof {}  (not implemented in this build)",
            ProofKind::Contrast
        ),
    ];

    Ok(ImplementationContract {
        schema_version: CONTRACT_SCHEMA_VERSION,
        generated_at: Timestamp::now(),
        generated_by: format!("vds impl {id}"),
        component: record.id.clone(),
        name: record.name.clone(),
        status: record.status,
        record_digest: Digest::of_file(&located.path)?,
        read_the_design_from: read_from,
        write_the_code_to: write_to,
        must,
        not_checked,
        judged_by,
    })
}

impl ImplementationContract {
    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "# Implementation contract: {} `{}`\n\n",
            self.name, self.component
        ));
        out.push_str(&format!(
            "Status `{}`, record digest `{}`.\n\n",
            self.status, self.record_digest
        ));
        out.push_str(
            "This is the contract your implementation will be judged against. It states \
             requirements and no realisations: it says what must be true, never what it must \
             look like.\n\n",
        );

        out.push_str("## Read the design from\n\n");
        match (
            &self.read_the_design_from.figma_file,
            &self.read_the_design_from.figma_node,
        ) {
            (Some(file), Some(node)) => {
                out.push_str(&format!("- Figma file `{file}`, node `{node}`"));
                match self.read_the_design_from.resolved {
                    Some(true) => out.push_str(" (measured: resolves)\n"),
                    Some(false) => out.push_str(" (measured: **DOES NOT RESOLVE**)\n"),
                    None => out.push_str(" (not measured)\n"),
                }
            }
            _ => out.push_str("- **No Figma node recorded.**\n"),
        }
        out.push_str(&format!("\n{}\n\n", self.read_the_design_from.note));

        out.push_str("## Write the code to\n\n");
        match &self.write_the_code_to {
            Some(target) => {
                out.push_str(&format!("- `{}`\n", target.source_file));
                out.push_str(&format!(
                    "- exported as `{}`, imported from `{}`\n",
                    target.export_name, target.import_path
                ));
                out.push_str(&format!(
                    "- the file {}\n\n",
                    if target.exists {
                        "exists"
                    } else {
                        "**does not exist yet**"
                    }
                ));
            }
            None => out.push_str("- **No code counterpart recorded.**\n\n"),
        }

        out.push_str(&format!("## Requirements ({})\n\n", self.must.len()));
        for requirement in &self.must {
            out.push_str(&format!("- [ ] {}\n", requirement.what));
            out.push_str(&format!("      basis: {}\n", requirement.basis));
            match &requirement.checked_by {
                Some(kind) => out.push_str(&format!("      checked by: `{kind}`\n")),
                None => out.push_str("      checked by: **nothing in this build**\n"),
            }
        }

        out.push_str("\n## What no check will catch\n\n");
        for item in &self.not_checked {
            out.push_str(&format!("- {item}\n"));
        }

        out.push_str("\n## Run these before you call it done\n\n");
        for command in &self.judged_by {
            out.push_str(&format!("- `{command}`\n"));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Fixture;

    #[test]
    fn a_contract_lists_every_prop_state_and_floor_as_a_requirement() {
        let f = Fixture::new();
        let id = f.register("Button", Status::Registered);
        let store = f.store();
        let contract = build(&store, &id, None).unwrap();
        assert!(contract.must.iter().any(|r| r.what.contains("default")));
        assert!(
            contract
                .must
                .iter()
                .any(|r| r.what.contains("control-border"))
        );
        assert!(contract.must.iter().any(|r| r.what.contains("export")));
    }

    #[test]
    fn a_requirement_nothing_checks_says_so_rather_than_being_omitted() {
        let f = Fixture::new();
        let id = f.register("Button", Status::Registered);
        f.amend(&id, |r| {
            r.a11y.keyboard = vec![vds_core::KeyboardContract {
                key: "Enter".into(),
                effect: "activates".into(),
            }];
        });
        let store = f.store();
        let contract = build(&store, &id, None).unwrap();
        let keyboard = contract
            .must
            .iter()
            .find(|r| r.what.contains("Enter"))
            .unwrap();
        assert!(
            keyboard.checked_by.is_none(),
            "a keyboard contract is checked by nothing in this build, and listing it as \
             checked would be a claim the build cannot support"
        );
    }

    /// The "checked by nothing" paragraph is DERIVED from
    /// `unimplemented_because`, so it appears exactly when a kind is unbuilt and
    /// vanishes when it is built.
    ///
    /// It used to assert the paragraph was present, naming `parity`. All ten
    /// kinds are now implemented, so its absence is the correct state and the
    /// test asserts the derivation instead: no kind is claimed unbuilt, and if
    /// one ever is, the contract names it rather than staying silent.
    #[test]
    fn the_contract_claims_a_proof_is_unbuilt_only_where_it_is() {
        let f = Fixture::new();
        let id = f.register("Button", Status::Registered);
        let store = f.store();
        let contract = build(&store, &id, None).unwrap();

        for kind in ProofKind::ALL {
            let claimed = contract
                .not_checked
                .iter()
                .any(|n| n.contains(kind.as_str()) && n.contains("checked by nothing"));
            assert_eq!(
                claimed,
                !kind.is_implemented(),
                "the contract says {kind} is checked by nothing and it is implemented, or the \
                 reverse: {:?}",
                contract.not_checked
            );
        }

        // The one thing no accumulation of proofs replaces is still said.
        assert!(
            contract
                .not_checked
                .iter()
                .any(|n| n.contains("Whether the result looks right")),
            "{:?}",
            contract.not_checked
        );
    }

    #[test]
    fn a_required_state_not_yet_drawn_is_flagged_in_the_requirement_itself() {
        let f = Fixture::new();
        let id = f.register("Button", Status::Registered);
        f.amend(&id, |r| {
            r.states.required = vec![State::Default, State::Focus];
            r.states.drawn = vec![State::Default];
        });
        let store = f.store();
        let contract = build(&store, &id, None).unwrap();
        let focus = contract
            .must
            .iter()
            .find(|r| r.what.contains("focus"))
            .unwrap();
        assert!(focus.what.contains("NOT yet drawn"), "{}", focus.what);
    }

    #[test]
    fn a_retired_component_has_no_lawful_contract() {
        let f = Fixture::new();
        let id = f.register("Old", Status::Registered);
        f.amend(&id, |r| r.status = Status::Retired);
        let store = f.store();
        let error = build(&store, &id, None).unwrap_err();
        assert!(error.to_string().contains("VDS S-9(8)"), "{error}");
    }

    #[test]
    fn a_contract_says_when_the_node_it_points_at_was_never_measured() {
        let f = Fixture::new();
        let id = f.register("Button", Status::Registered);
        f.amend(&id, |r| {
            r.figma = Some(vds_core::FigmaNode {
                file_key: "KEY".into(),
                node_id: "12:34".into(),
                captured_at: Timestamp::fixed(2026, 7, 25, 10, 0, 0),
            });
        });
        let store = f.store();
        let contract = build(&store, &id, None).unwrap();
        assert_eq!(contract.read_the_design_from.resolved, None);
        assert!(
            contract
                .read_the_design_from
                .note
                .contains("vds figma pull")
        );
    }

    #[test]
    fn a_contract_reports_a_node_measured_not_to_resolve() {
        let f = Fixture::new();
        let id = f.register("Button", Status::Registered);
        f.amend(&id, |r| {
            r.figma = Some(vds_core::FigmaNode {
                file_key: "KEY".into(),
                node_id: "12:34".into(),
                captured_at: Timestamp::fixed(2026, 7, 25, 10, 0, 0),
            });
        });
        let mut ledger = crate::testing::figma_ledger(&[(&id, vec![])]);
        ledger.nodes[0].resolved = false;
        ledger.nodes[0].unresolved_because = Some("no node 12:34 in the file".into());
        ledger.content_digest = ledger.compute_content_digest().unwrap();

        let store = f.store();
        let contract = build(&store, &id, Some(&ledger)).unwrap();
        assert_eq!(contract.read_the_design_from.resolved, Some(false));
        assert!(contract.to_markdown().contains("DOES NOT RESOLVE"));
    }

    /// VDS S-2(4): the contract is handed to an implementer, so a realisation in
    /// it would be VDS handing down a design value.
    #[test]
    fn a_contract_carries_no_realisation() {
        let f = Fixture::new();
        let id = f.register("Button", Status::Registered);
        let store = f.store();
        let contract = build(&store, &id, None).unwrap();
        let text = contract.to_markdown() + &serde_yaml::to_string(&contract).unwrap();
        if let Some(found) = crate::testing::realisation_in(&text) {
            panic!("the contract contains {found:?}, which is a realisation (VDS S-2(4))");
        }
    }

    #[test]
    fn the_markdown_is_a_checklist_an_implementer_can_work_through() {
        let f = Fixture::new();
        let id = f.register("Button", Status::Registered);
        let store = f.store();
        let markdown = build(&store, &id, None).unwrap().to_markdown();
        assert!(markdown.contains("- [ ] "));
        assert!(markdown.contains("What no check will catch"));
        assert!(markdown.contains("Run these before you call it done"));
    }
}
