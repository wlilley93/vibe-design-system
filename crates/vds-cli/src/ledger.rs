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
