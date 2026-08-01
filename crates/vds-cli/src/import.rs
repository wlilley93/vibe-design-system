//! `vds register import`: scaffold candidate records from the codebase.
//!
//! VDS S-14(1) is honest that the register is the expensive part, and that it
//! costs the same whether or not VDS exists. That is true, and it is also true
//! that a project facing ninety records before any proof can say anything will
//! not write them, and a register that never exists is the state that produced
//! both defects at VDS S-1(4).
//!
//! This turns a week of typing into a review pass. Three rules keep it from
//! turning into a register that agrees with the code by construction and
//! therefore checks nothing:
//!
//!   1. **Everything is minted at `proposed`.** A candidate is not a contract.
//!      VDS S-5(4) makes the lifecycle a directed path exactly so that a record
//!      arrives at `registered` because someone decided it should, and
//!      `composition` refuses a `proposed` record, so importing alone unlocks
//!      nothing.
//!   2. **Nothing is overwritten, ever.** An existing record is the reviewed
//!      one; a scan of the code is not evidence that it is wrong.
//!   3. **Every guess is labelled.** An import path taken from a screen that
//!      really imports the component is a MEASUREMENT; one derived from a naming
//!      rule is a guess, and the report says which each row is.
//!
//! What it deliberately does NOT fill in: required states, contrast floors,
//! roles and keyboard contracts. Those are the contract, and the contract is a
//! decision. Inventing a plausible one is how a register comes to describe what
//! the code already does rather than what it must do, and a register that
//! describes the code cannot disagree with it.

use clap::Args as ClapArgs;
use vds_core::{
    Accessibility, CodeCounterpart, ComponentId, ComponentRecord, Demand, NameSource, PropContract,
    Result, StateContract, Status, Timestamp, VdsError,
};
use vds_scan::library::{ImportPathSource, LibraryExport, import_path_for, scan_library};
use vds_store::Store;

use crate::{Context, PASSED};

#[derive(ClapArgs)]
pub struct Args {
    /// Report what would be written, and write nothing.
    #[arg(long)]
    dry_run: bool,
    /// Import a component whose import path could only be DERIVED, not observed.
    ///
    /// Off by default: a derived path is a guess, and a register full of guessed
    /// coordinates fails `reconciliation` in a way that looks like the code's
    /// fault.
    #[arg(long)]
    include_derived: bool,
}

pub fn run(ctx: &Context, args: &Args) -> Result<i32> {
    let project = ctx.project()?;
    let store = Store::new(&project);

    if project.config.surface.library_dirs.is_empty() {
        return Err(VdsError::precondition(
            "[surface] library_dirs is empty, so there is no library to import from. Name the \
             directories your components live in.",
        ));
    }

    let scan = scan_library(&project)?;
    // The screens ledger is EVIDENCE for the import path. Absent is fine, and
    // means every path has to be derived, which the report says.
    let ledger = match vds_scan::load_fresh(&project) {
        Ok(ledger) => Some(ledger),
        Err(error) => {
            println!(
                "note: no usable screens ledger, so every import path below is DERIVED rather \
                 than observed."
            );
            println!("      {error}");
            println!();
            None
        }
    };

    let existing = store.read_register()?;
    let claimed: Vec<(String, String)> = existing
        .iter()
        .filter_map(|r| {
            r.value
                .code
                .as_ref()
                .map(|c| (c.import_path.clone(), c.export_name.clone()))
        })
        .collect();
    let by_source: Vec<String> = existing
        .iter()
        .filter_map(|r| r.value.code.as_ref().map(|c| c.source_file.clone()))
        .collect();

    let mut to_write: Vec<(ComponentRecord, ImportPathSource, &LibraryExport)> = Vec::new();
    let mut already = 0usize;
    let mut deferred: Vec<(&LibraryExport, String)> = Vec::new();

    for export in &scan.exports {
        let source = import_path_for(&project, export, ledger.as_ref());
        let Some(specifier) = source.specifier() else {
            deferred.push((
                export,
                "no screen imports it and no governed prefix could be applied, so there is no \
                 coordinate to register it under"
                    .to_owned(),
            ));
            continue;
        };

        if claimed
            .iter()
            .any(|(path, name)| path == specifier && name == &export.export_name)
            || by_source.contains(&export.source_file)
        {
            already += 1;
            continue;
        }

        if matches!(source, ImportPathSource::Derived { .. }) && !args.include_derived {
            deferred.push((
                export,
                format!(
                    "its import path is DERIVED as {specifier:?} and no screen imports it, so \
                     the coordinate is a guess. Pass --include-derived to take it anyway."
                ),
            ));
            continue;
        }

        to_write.push((candidate(export, specifier)?, source, export));
    }

    // Allocate ids only once the set is final, and allocate them in order, so a
    // dry run and the real run describe the same thing.
    let mut next = ComponentId::allocate(&store.register_dir())?;
    let mut planned = Vec::new();
    for (mut record, source, export) in to_write {
        record.id = next.clone();
        next = ComponentId::parse(format!(
            "CMP-{:04}",
            next.as_str()
                .trim_start_matches("CMP-")
                .parse::<u32>()
                .unwrap_or(0)
                + 1
        ))?;
        planned.push((record, source, export));
    }

    // Files and exports are counted separately, because they are not the same number and
    // this line used to add them together and call the sum "library files". It read
    // correctly only while every file yielded at most one export - true for the React
    // convention this scanner was written against, and false the moment it learned to read
    // a CommonJS registry, where one file exports a variant per key. The count then jumped
    // from 13 to 26 with no file added, which is how the mislabel was found.
    let files: std::collections::BTreeSet<&str> = scan
        .exports
        .iter()
        .map(|e| e.source_file.as_str())
        .chain(scan.skipped.iter().map(|s| s.path.as_str()))
        .collect();
    println!(
        "scanned {} library files: {} exported components across {} of them, {} files skipped",
        files.len(),
        scan.exports.len(),
        files.len() - scan.skipped.len(),
        scan.skipped.len()
    );
    println!("  already registered: {already}");
    println!("  to import:          {}", planned.len());
    println!("  deferred:           {}", deferred.len());
    println!();

    for (record, source, export) in &planned {
        let provenance = match source {
            ImportPathSource::Observed { route, .. } => format!("OBSERVED in {route}"),
            ImportPathSource::Derived { .. } => "DERIVED from a naming rule, unverified".into(),
            ImportPathSource::Unknown => "unknown".into(),
        };
        println!("  {} {}", record.id, record.name);
        println!("    source:      {}", export.source_file);
        println!(
            "    coordinate:  {}::{}  ({provenance})",
            record
                .code
                .as_ref()
                .map(|c| c.import_path.as_str())
                .unwrap_or(""),
            record
                .code
                .as_ref()
                .map(|c| c.export_name.as_str())
                .unwrap_or("")
        );
        if record.props.is_empty() {
            println!(
                "    props:       none found. {}",
                export
                    .props_incomplete_because
                    .as_deref()
                    .unwrap_or("no reason recorded")
            );
        } else {
            println!(
                "    props:       {}",
                record
                    .props
                    .iter()
                    .map(|p| format!("{}{}", p.name, if p.required { "" } else { "?" }))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            // A NON-EMPTY list can still be a subset, and this arm used to say
            // nothing at all - the reason was computed, carried on the export and
            // then printed only when the list was empty. So a component whose
            // declaration intersects `React.HTMLAttributes` was reported as
            // `props: interactive?, selected?` with no hint that it also accepts
            // every DOM attribute. `parity` knew (it skips such a row rather than
            // crediting it), but the person reading the import did not, and they
            // are the one deciding whether the candidate is ready to advance.
            if let Some(because) = &export.props_incomplete_because {
                println!("    INCOMPLETE:  {because}");
            }
        }
    }

    if !deferred.is_empty() {
        println!();
        println!("Deferred, with the reason. None of these is an error:");
        for (export, why) in &deferred {
            println!("  {} ({})", export.export_name, export.source_file);
            println!("    {why}");
        }
    }

    if !scan.skipped.is_empty() {
        println!();
        println!("Skipped files, named so the carve-out is a list you can disagree with:");
        for skipped in &scan.skipped {
            println!("  {}: {}", skipped.path, skipped.because);
        }
    }

    if args.dry_run {
        println!();
        println!("--dry-run: nothing was written.");
        return Ok(PASSED);
    }

    for (record, _, _) in &planned {
        // `create` refuses to overwrite, so a race or a mistaken id is a
        // fail-closed error and never a lost record (VDS S-4(4)).
        store.create(&store.record_path(&record.id), record)?;
    }

    println!();
    println!("wrote {} candidate records at `proposed`.", planned.len());
    println!();
    println!("These are CANDIDATES, not contracts. Every one of them:");
    println!("  - carries no required states, no contrast floors, no role and no keyboard");
    println!("    contract. Those are the contract, and the contract is a decision. Filling");
    println!("    them in from the code would make the register describe what the code does");
    println!("    rather than what it must do, and a register that describes the code cannot");
    println!("    disagree with it.");
    println!("  - sits at `proposed`, which `composition` refuses, so importing unlocks");
    println!("    nothing on its own (VDS S-5(4), S-6(2)).");
    println!();
    println!("Read each one, add its contract, then advance it:");
    println!("  vds register show <id>");
    println!("  vds register amend <id> --kind non_breaking --what \"...\" --add-required focus");
    println!("  vds register set-status <id> designed");
    println!("  vds register set-status <id> registered");
    Ok(PASSED)
}

/// A candidate record: what the code says, and nothing the code cannot say.
fn candidate(export: &LibraryExport, specifier: &str) -> Result<ComponentRecord> {
    let name = export
        .local_name
        .clone()
        .unwrap_or_else(|| export.export_name.clone());
    let now = Timestamp::now();
    Ok(ComponentRecord {
        // Replaced by the caller once the set is final.
        id: ComponentId::parse("CMP-0001")?,
        name,
        status: Status::Proposed,
        contract_version: 1,
        figma: None,
        code: Some(CodeCounterpart {
            import_path: specifier.to_owned(),
            source_file: export.source_file.clone(),
            export_name: export.export_name.clone(),
        }),
        props: export
            .props
            .iter()
            .map(|p| PropContract {
                name: p.name.clone(),
                type_expr: p.type_expr.clone(),
                required: p.required,
                figma_property: None,
            })
            .collect(),
        // Empty, deliberately. See the note printed after a write.
        states: StateContract::default(),
        a11y: Accessibility {
            role: None,
            accessible_name_source: NameSource::Children,
            keyboard: vec![],
            contrast_floors: vec![],
        },
        demand: Demand {
            routes: 0,
            measured_at: now.clone(),
            measured_by: "vds register import (not yet measured; run: vds register \
                          measure-demand --all)"
                .into(),
        },
        supersedes: vec![],
        superseded_by: None,
        amendments: vec![],
        basis: vec!["ACT-VDS-001:s5".into()],
        measured_by: vec![],
        directed_at: None,
        grace_days: None,
        deprecated_at: None,
        retired_at: None,
        retirement_proof_id: None,
        notes: Some(format!(
            "Imported by `vds register import` from {}:{}. A CANDIDATE, not a contract: it \
             carries what the code says and nothing the code cannot say. Its required states, \
             contrast floors, role and keyboard contract are decisions nobody has made yet.",
            export.source_file, export.line
        )),
    })
}
