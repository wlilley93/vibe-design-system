//! `vds ledger`: regenerate a generated inventory.

use std::collections::{BTreeMap, BTreeSet};

use clap::{Args as ClapArgs, Subcommand};
use vds_core::{Result, VdsError};

use crate::{Context, PASSED};

#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    which: Which,
}

#[derive(Subcommand)]
enum Which {
    /// The declared surface: every screen matching `[surface] screen_globs`.
    Screens,
    /// The SHAPE the shipped stylesheet gives its surfaces (VDS S-7A(4)).
    ///
    /// The input `geometry` measures a bound against, and the only thing that
    /// makes the twelfth kind non-vacuous. It is a generator and not a hand
    /// edit for the reason VDS S-4(2) gives: the reading is this proof's SOLE
    /// measurement, so a hand-written one is a number nobody derived, and a
    /// hand-EDITED one turns a bound being exceeded into a bound being met.
    Geometry {
        /// A measurement produced by the SUBJECT's own reader, as JSON.
        ///
        /// Without it, VDS's built-in stylesheet reader runs. With it, VDS takes
        /// the counts and owns only the ledger format and the digest.
        ///
        /// This seam exists because VDS must not decide what compliance MEANS.
        /// A project composing shape from utility classes in its markup
        /// (`rounded-lg`, `p-4`) puts it somewhere no stylesheet reader can see,
        /// and the built-in reader says so on every run rather than reporting a
        /// confident floor. That project writes its own reader; VDS still refuses
        /// a reading that does not witness its own content, which is the half
        /// that has to be uniform.
        ///
        /// Same shape as `vds pin generate --from` and `vds ledger ci --from`:
        /// derive from saved bytes, so the derivation is reproducible and the
        /// network call stays outside the proof (VDS S-7(2)(1)).
        #[arg(long, value_name = "PATH")]
        from: Option<std::path::PathBuf>,
    },
    /// The measured values of every pinned metric (draft S-7C).
    ///
    /// Always `--from`: what a metric MEANS is the subject's reader talking,
    /// and VDS deliberately has no built-in counter to substitute for it. The
    /// digest is computed here, never taken from the file, for the reason the
    /// geometry arm gives.
    Burndown {
        /// A reading produced by the subject's own reader, as JSON.
        #[arg(long, value_name = "PATH")]
        from: std::path::PathBuf,
    },
    /// The estate's ROUTE MANIFEST: what a stage-4 visual pass is supposed to
    /// cover (draft S-7D(9)).
    ///
    /// Always `--from`: WHICH routes are in the programme is the estate's
    /// question - in the motivating project, a route tracker - and VDS
    /// deciding it would make VDS the authority on the estate's own scope.
    /// What VDS owns is that the enumeration exists, is digest-witnessed, and
    /// is reported against in three populations.
    Routes {
        /// The estate's list, as JSON in the route-manifest shape.
        #[arg(long, value_name = "PATH")]
        from: std::path::PathBuf,
    },
    /// The authority snapshot binding the shipped geometry reading to a signed
    /// frame's decided values (draft S-7A(5)).
    ///
    /// Always `--from`: the comparison of decided values against shipped ones
    /// is the subject's comparator talking, run out of band over a SAVED REST
    /// capture, because a proof may not call the network (VDS S-7(2)(1)).
    GeometryAuthority {
        /// A snapshot produced by the subject's comparator, as JSON.
        #[arg(long, value_name = "PATH")]
        from: std::path::PathBuf,
    },
    /// Whether the workflow the lock names has ever actually concluded (BREACH-0011).
    ///
    /// D4 asks "is every gate invoked by CI". It read the lock's own declaration, then
    /// the workflow FILE, and never the RUN - so it reported Met over seventeen gates
    /// while the job had never once started. A conclusion is a network fact and
    /// VDS S-7(2)(1) forbids a network call inside a proof, so it is recorded here and
    /// `--from` makes the derivation reproducible from saved bytes.
    Ci {
        /// A saved `gh run list --json conclusion,createdAt,headSha,workflowName` response.
        /// Without it, `gh` is invoked - which needs the credential and the network.
        #[arg(long, value_name = "PATH")]
        from: Option<std::path::PathBuf>,
        /// The workflow as the LOCK spells it, because that is the key D4 looks up. The
        /// forge reports no path at all, so the join is this path -> the `name:` inside
        /// that file -> the forge's `workflowName`.
        #[arg(long, default_value = ".github/workflows/vds-enforce.yml")]
        workflow: String,
        /// How many runs to ask for. It becomes `runs_considered`, so a narrow window is
        /// visible in the ledger as a narrow window.
        #[arg(long, default_value_t = 60)]
        limit: u32,
    },
}

pub fn run(ctx: &Context, args: &Args) -> Result<i32> {
    let project = ctx.project()?;
    match &args.which {
        Which::Ci {
            from,
            workflow,
            limit,
        } => {
            let store = vds_store::Store::new(&project);
            let fields = "conclusion,createdAt,headSha,workflowName";

            // The forge names runs after the workflow's declared `name:`, and reports no
            // path at all - `gh run list --json path` is refused by name, which is how
            // this was found. So read the name out of the file rather than guessing it
            // from the filename: `vds-enforce.yml` need not declare `name: vds-enforce`,
            // and a guess that happens to be right today is a join that breaks silently.
            let wf_path = project.root.join(workflow);
            let declared_name: Option<String> = std::fs::read_to_string(&wf_path)
                .ok()
                .and_then(|t| serde_yaml::from_str::<serde_yaml::Value>(&t).ok())
                .and_then(|d| d.get("name").and_then(|n| n.as_str()).map(str::to_owned));
            if declared_name.is_none() {
                println!(
                    "note: {} declares no `name:` (or does not parse), so no name filter \
                     will be applied and every run in the response is counted.",
                    project.rel(&wf_path)
                );
            }
            let (raw, source) = match from {
                Some(path) => {
                    let text = std::fs::read_to_string(path)
                        .map_err(|e| vds_core::VdsError::io(path.display(), e))?;
                    (text, project.rel(path))
                }
                None => {
                    // Shelling out rather than speaking HTTP, for the same reason
                    // `figma pull` shells out to curl: the credential, the retries and
                    // the pagination are the forge CLI's problem, not the kernel's.
                    let file = std::path::Path::new(workflow)
                        .file_name()
                        .and_then(|f| f.to_str())
                        .unwrap_or(workflow.as_str());
                    let cmd =
                        format!("gh run list --workflow={file} --limit {limit} --json {fields}");
                    let out = std::process::Command::new("gh")
                        .args(["run", "list", "--workflow", file])
                        .args(["--limit", &limit.to_string()])
                        .args(["--json", fields])
                        .current_dir(&project.root)
                        .output()
                        .map_err(|e| {
                            vds_core::VdsError::precondition(format!(
                                "could not run `gh`: {e}. Either install it and authenticate, \
                                 or pass --from with a saved response - which needs no \
                                 credential and makes the derivation reproducible."
                            ))
                        })?;
                    if !out.status.success() {
                        return Err(vds_core::VdsError::precondition(format!(
                            "`{cmd}` failed: {}",
                            String::from_utf8_lossy(&out.stderr).trim()
                        )));
                    }
                    (String::from_utf8_lossy(&out.stdout).into_owned(), cmd)
                }
            };

            let ledger = crate::ci::derive(
                workflow,
                declared_name.as_deref(),
                &source,
                &raw,
                "vds ledger ci",
            )?;
            let path = crate::ci::write(&store, &ledger)?;
            let row = ledger
                .row(workflow)
                .expect("derive always writes the row it was asked for");

            println!("wrote {}", project.rel(&path));
            println!("  workflow:         {}", row.file);
            println!("  runs considered:  {}", row.runs_considered);
            println!("  successes:        {}", row.successes);
            if let Some(c) = &row.newest_conclusion {
                println!(
                    "  newest:           {c}{}",
                    row.newest_at
                        .as_deref()
                        .map(|a| format!(" at {a}"))
                        .unwrap_or_default()
                );
            }
            for (name, n) in &row.conclusions {
                println!("    {name}: {n}");
            }
            for note in &ledger.notes {
                println!("  note: {note}");
            }
            // Deliberately NOT an exit code. This command records; D4 judges. A generator
            // that also failed the build would make the record something people avoid
            // regenerating, which is how a ledger goes stale.
            Ok(PASSED)
        }
        Which::Geometry { from } => {
            let mut reading = match from {
                None => vds_scan::geometry::build(&project, vds_core::Timestamp::now())?,
                Some(path) => {
                    let text = std::fs::read_to_string(path)
                        .map_err(|e| VdsError::io(path.display(), e))?;
                    let mut supplied: vds_core::GeometryReading = serde_json::from_str(&text)
                        .map_err(|e| VdsError::Artefact {
                            path: project.rel(path),
                            message: format!("is not a geometry reading: {e}"),
                        })?;
                    // The DIGEST is computed here and never taken from the file.
                    // A subject that supplied its own could supply a wrong one,
                    // and the whole value of the field is that it was computed
                    // from the content by something other than the hand that
                    // wrote the content.
                    supplied.content_digest = supplied.compute_content_digest()?;
                    supplied
                }
            };
            // Recomputed unconditionally, so both paths leave the file in a state
            // `geometry` R10 and `ledger_staleness` R5 accept.
            reading.content_digest = reading.compute_content_digest()?;
            let path = vds_core::write_reading(&project, &reading)?;
            println!("wrote {}", project.rel(&path));
            println!("  by:        {}", reading.generated_by);
            println!("  read:      {}", reading.sources.join(", "));
            println!("  taken at:  {}", reading.taken_at);
            println!("  from:      {}", reading.read_from);
            if reading.kinds.is_empty() {
                println!();
                println!(
                    "  NO SHAPE DECLARATION WAS FOUND AT ALL. That is a finding, not an \
                     empty result: either the stylesheet sets no radius, boundary weight, \
                     padding or type step anywhere, or this project composes shape from \
                     utility classes in the markup, which this reader does not see. The \
                     second is far more likely, and on such a project a bound declared \
                     against this reading would be a bound over nothing."
                );
            }
            for kind in &reading.kinds {
                println!(
                    "  {:16} {:>4} considered, {:>4} non-compliant, {:>4} undecided",
                    kind.surface_kind.to_string(),
                    kind.considered,
                    kind.non_compliant,
                    kind.undecided
                );
                for sample in kind.sample.iter().take(4) {
                    println!("      {sample}");
                }
            }
            println!();
            println!("  Does NOT cover:");
            for line in &reading.does_not_cover {
                println!("    - {line}");
            }
            Ok(PASSED)
        }
        Which::Routes { from } => {
            let text =
                std::fs::read_to_string(from).map_err(|e| VdsError::io(from.display(), e))?;
            let mut manifest: vds_core::RouteManifest =
                serde_json::from_str(&text).map_err(|e| VdsError::Artefact {
                    path: project.rel(from),
                    message: format!("is not a route manifest: {e}"),
                })?;
            if manifest.routes.is_empty() {
                return Err(VdsError::precondition(
                    "the manifest enumerates no route. An enumeration of nothing makes every \
                     coverage population zero and every never-reviewed route invisible, which \
                     is the defect this ledger exists to close (draft S-7D(9)).",
                ));
            }
            let before = manifest.routes.len();
            manifest.routes.sort();
            manifest.routes.dedup();
            // Computed here, never taken from the file: a route quietly removed
            // from the manifest is a route that stops being reported as owed,
            // and the digest is what makes that edit visible.
            manifest.content_digest = manifest.compute_content_digest()?;
            let path = vds_core::write_route_manifest(&project, &manifest)?;
            println!("wrote {}", project.rel(&path));
            println!("  source:   {}", manifest.source);
            println!("  taken at: {}", manifest.taken_at);
            println!(
                "  routes:   {}{}",
                manifest.routes.len(),
                if manifest.routes.len() < before {
                    format!(
                        " ({} duplicate(s) collapsed)",
                        before - manifest.routes.len()
                    )
                } else {
                    String::new()
                }
            );
            for line in &manifest.does_not_cover {
                println!("  does NOT cover: {line}");
            }
            println!();
            println!(
                "`vds proof visual_review` now reports every one of these routes in one of \
                 three populations: current, owed by drift, or never reviewed. A route missing \
                 from this list is a route nothing will report as owed."
            );
            Ok(PASSED)
        }
        Which::Burndown { from } => {
            let text =
                std::fs::read_to_string(from).map_err(|e| VdsError::io(from.display(), e))?;
            let mut reading: vds_core::BurndownReading =
                serde_json::from_str(&text).map_err(|e| VdsError::Artefact {
                    path: project.rel(from),
                    message: format!("is not a burndown reading: {e}"),
                })?;
            // Computed here, never taken from the file: the whole value of the
            // digest is that something other than the hand that wrote the
            // content computed it.
            reading.content_digest = reading.compute_content_digest()?;
            let path = vds_core::write_burndown_reading(&project, &reading)?;
            println!("wrote {}", project.rel(&path));
            println!("  by:       {}", reading.generated_by);
            println!("  taken at: {}", reading.taken_at);
            for row in &reading.rows {
                println!(
                    "  {:32} {:>8}{}",
                    row.metric,
                    row.value,
                    row.measured_by
                        .as_deref()
                        .map(|m| format!("   ({m})"))
                        .unwrap_or_default()
                );
            }
            Ok(PASSED)
        }
        Which::GeometryAuthority { from } => {
            let text =
                std::fs::read_to_string(from).map_err(|e| VdsError::io(from.display(), e))?;
            let mut snapshot: vds_core::GeometryAuthority =
                serde_json::from_str(&text).map_err(|e| VdsError::Artefact {
                    path: project.rel(from),
                    message: format!("is not a geometry authority snapshot: {e}"),
                })?;
            // The two input hashes are REFUSED unless they match what is on
            // disk right now: a snapshot born stale would fail the proof on
            // its first run and teach the subject the ledger is noise.
            let capture_path = project.root.join(&snapshot.capture);
            let capture_now = vds_core::Digest::of_file(&capture_path).map_err(|_| {
                VdsError::precondition(format!(
                    "the capture {} cannot be read, so the authority side's input hash                      cannot be verified. The capture lives in the PROJECT tree (it holds                      realisations and may not enter .vds/), and the snapshot names it.",
                    snapshot.capture
                ))
            })?;
            if capture_now != snapshot.capture_digest {
                return Err(VdsError::precondition(format!(
                    "the snapshot binds capture digest {} and {} digests to {capture_now} on                      disk. A snapshot born stale proves nothing; regenerate it from the                      current capture.",
                    snapshot.capture_digest, snapshot.capture
                )));
            }
            let reading = vds_core::read_reading(&project)?.ok_or_else(|| {
                VdsError::precondition(
                    "no geometry reading exists, so there is no artefact side to bind. Run                      `vds ledger geometry` first.",
                )
            })?;
            if reading.content_digest != snapshot.reading_digest {
                return Err(VdsError::precondition(format!(
                    "the snapshot binds reading digest {} and the reading on disk digests to                      {}. Regenerate the snapshot against the current reading.",
                    snapshot.reading_digest, reading.content_digest
                )));
            }
            // [2026] VJS-CA-VDS 1 order 7: the comparator is refused on the
            // same terms as a stale capture or reading. The agreement rows are
            // its assertion, and a snapshot naming a program that cannot be
            // read - or one that has moved since the comparison - is bound to
            // an input that no longer exists.
            let comparator_path = project.root.join(&snapshot.comparator);
            let comparator_now = vds_core::Digest::of_file(&comparator_path).map_err(|_| {
                VdsError::precondition(format!(
                    "the comparator {} cannot be read. The engine cannot re-derive an \
                     agreement bit (that needs the values, which VDS S-2(2) forbids it to \
                     hold), so the program that produced them is the only witness there is \
                     ([2026] VJS-CA-VDS 1 order 7).",
                    snapshot.comparator
                ))
            })?;
            if comparator_now != snapshot.comparator_digest {
                return Err(VdsError::precondition(format!(
                    "the snapshot binds comparator digest {} and {} digests to \
                     {comparator_now} on disk. Regenerate the snapshot with the comparator \
                     that actually ran.",
                    snapshot.comparator_digest, snapshot.comparator
                )));
            }
            snapshot.content_digest = snapshot.compute_content_digest()?;
            let path = vds_core::write_authority(&project, &snapshot)?;
            println!("wrote {}", project.rel(&path));
            println!("  frame:    {}/{}", snapshot.file_key, snapshot.node_id);
            println!(
                "  capture:  {} (fetched {})",
                snapshot.capture, snapshot.fetched_at
            );
            for row in &snapshot.rows {
                println!(
                    "  {:16} {}",
                    row.surface_kind.to_string(),
                    match row.agrees {
                        vds_core::AgreementState::Agrees => "agrees",
                        vds_core::AgreementState::Disagrees => "DISAGREES",
                        vds_core::AgreementState::NotDrawn => "not drawn by the frame",
                    }
                );
            }
            Ok(PASSED)
        }
        Which::Screens => {
            let (path, ledger) = vds_scan::write(&project)?;
            let components = ledger.component_references().count();
            let elements: usize = ledger
                .screens
                .iter()
                .flat_map(|s| &s.references)
                .filter(|r| r.kind == vds_scan::ReferenceKind::Element)
                .count();
            let unresolved = ledger
                .component_references()
                .filter(|(_, r)| r.import_path.is_none())
                .count();

            println!("wrote {}", project.rel(&path));
            println!("  screens:                 {}", ledger.screens.len());
            println!("  component references:    {components}");
            println!(
                "  bare element references: {elements}  (informational only, \
                 VDS S-9(10) RESERVED)"
            );
            if unresolved > 0 {
                println!(
                    "  unresolved imports:      {unresolved}  (defined locally, or bound by \
                     more than one import)"
                );
            }
            println!("  source_digest:           {}", ledger.source_digest);
            println!("  content_digest:          {}", ledger.content_digest);

            // A count is a number; a name repeated across screens is a work
            // list. VDS exists to confine design to the library, and a component
            // defined inline in a page is the most ordinary way that fails: it
            // is outside every proof, because no register coordinate can name
            // it, and nothing has ever said which ones they are.
            let mut inline: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
            for (screen, reference) in ledger.component_references() {
                if reference.import_path.is_none() {
                    inline
                        .entry(reference.root.as_str())
                        .or_default()
                        .insert(screen.route.as_str());
                }
            }
            if !inline.is_empty() {
                let mut ranked: Vec<(&&str, &BTreeSet<&str>)> = inline.iter().collect();
                ranked.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(b.0)));
                println!();
                println!(
                    "{} component names are defined inline rather than imported, across {} \
                     references.",
                    inline.len(),
                    unresolved
                );
                println!(
                    "  Each is outside every proof: no register coordinate can name a component \
                     that no module exports. The ones appearing on several screens are the \
                     candidates for the library."
                );
                for (name, routes) in ranked.iter().take(12) {
                    println!("    {:28} on {} screens", name, routes.len());
                }
                if ranked.len() > 12 {
                    println!("    ... and {} more", ranked.len() - 12);
                }
            }
            if ledger.screens.is_empty() {
                println!();
                println!(
                    "NOTE: the declared surface matched no file. Every VDS claim is bounded by \
                     this surface, so a surface of nothing makes every proof vacuous. Check \
                     [surface] screen_globs in .vds/config.toml."
                );
            }
            Ok(PASSED)
        }
    }
}
