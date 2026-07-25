//! `vds register`: the component register.
//!
//! Two rules from VDS S-5(4) and S-9 shape every subcommand here.
//!
//! **The lifecycle is a directed path and skipping is forbidden.** `add` may
//! therefore only mint a record at `proposed` or `designed`. The retired tool
//! let `--status verified` mint a component at the END of the lifecycle, and
//! composition then passed it, so the whole ordering that VDS S-6(2) calls "the
//! entire mechanism" could be stepped around with one flag.
//!
//! **Retirement is three phases that cannot be compressed** (VDS S-9(6)). So
//! `set-status` refuses to reach `deprecated` or `retired` at all, and each
//! phase has its own subcommand with its own preconditions.

use clap::{Args as ClapArgs, Subcommand};
use vds_core::{
    Accessibility, Amendment, AmendmentKind, CodeCounterpart, ComponentId, ComponentRecord,
    ContrastFloor, Demand, FigmaNode, FloorScope, KeyboardContract, NameSource, ProofId, ProofKind,
    PropContract, Result, State, StateContract, Status, Timestamp, VdsError, WarrantId,
    WarrantStatus, actor, breaking_reasons,
};
use vds_store::{Located, Store};

use crate::{Context, PASSED};

#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand)]
enum Action {
    /// Every record, with its status, contract version and measured demand.
    List,
    /// One record, in full.
    Show { id: String },
    /// Register a new component.
    Add(AddArgs),
    /// Scaffold CANDIDATE records from the codebase, at `proposed`.
    Import(crate::import::Args),
    /// Re-measure `demand` from the screens ledger.
    MeasureDemand(MeasureArgs),
    /// Amend a registered component's contract.
    Amend(AmendArgs),
    /// Advance a record one step along the lifecycle.
    SetStatus { id: String, status: String },
    /// Phase one of retirement: the supersession notice (VDS S-9(6)(1)).
    Deprecate(DeprecateArgs),
    /// Phase three of retirement: the tombstone (VDS S-9(6)(3)).
    Retire(RetireArgs),
}

pub fn run(ctx: &Context, args: &Args) -> Result<i32> {
    let project = ctx.project()?;
    let store = Store::new(&project);
    let _ = &ctx;
    match &args.action {
        Action::List => list(&store),
        Action::Show { id } => show(&store, id),
        Action::Add(a) => add(&store, a),
        Action::Import(a) => crate::import::run(ctx, a),
        Action::MeasureDemand(a) => measure(&store, a),
        Action::Amend(a) => amend(&store, a),
        Action::SetStatus { id, status } => set_status(&store, id, status),
        Action::Deprecate(a) => deprecate(&store, a),
        Action::Retire(a) => retire(&store, a),
    }
}

// ---------------------------------------------------------------- list, show

fn list(store: &Store) -> Result<i32> {
    let records = store.read_register()?;
    if records.is_empty() {
        println!("the register is empty");
        println!(
            "  Every VDS claim is bounded by what is registered, so an empty register makes \
             every proof vacuous rather than passing."
        );
        return Ok(PASSED);
    }
    let width = records
        .iter()
        .map(|r| r.value.name.len())
        .max()
        .unwrap_or(4);
    println!(
        "{} records in {}",
        records.len(),
        store.project.rel(&store.register_dir())
    );
    for record in &records {
        let code = record
            .value
            .code
            .as_ref()
            .map(|c| c.import_path.as_str())
            .unwrap_or("(unbuilt)");
        println!(
            "  {}  {:width$}  {:11}  v{}  routes={}  {code}",
            record.value.id,
            record.value.name,
            record.value.status.as_str(),
            record.value.contract_version,
            record.value.demand.routes,
        );
    }
    Ok(PASSED)
}

fn show(store: &Store, id: &str) -> Result<i32> {
    let id = ComponentId::parse(id)?;
    let record = store.read_record(&id)?;
    println!("# {}", store.project.rel(&record.path));
    print!(
        "{}",
        serde_yaml::to_string(&record.value).map_err(|e| VdsError::Serialize {
            what: id.to_string(),
            message: e.to_string(),
        })?
    );
    Ok(PASSED)
}

// ------------------------------------------------------------------------ add

#[derive(ClapArgs)]
pub struct AddArgs {
    #[arg(long)]
    name: String,
    /// Only `proposed` or `designed`. VDS S-5(4) makes the lifecycle a directed
    /// path, so a record cannot be minted part-way along it.
    #[arg(long, default_value = "proposed")]
    status: String,
    #[arg(long, requires_all = ["source_file", "export_name"])]
    import_path: Option<String>,
    #[arg(long, requires_all = ["import_path", "export_name"])]
    source_file: Option<String>,
    #[arg(long, requires_all = ["import_path", "source_file"])]
    export_name: Option<String>,
    /// `FILEKEY#12:34`. The node id is `<digits>:<digits>` or `<digits>-<digits>`.
    #[arg(long, value_name = "FILEKEY#NODE")]
    figma: Option<String>,
    /// Comma-separated states that are REQUIRED.
    #[arg(long)]
    require: Option<String>,
    /// Comma-separated states already DRAWN.
    #[arg(long)]
    drawn: Option<String>,
    /// Comma-separated states already BUILT.
    #[arg(long)]
    built: Option<String>,
    #[arg(long)]
    role: Option<String>,
    #[arg(long, default_value = "children")]
    name_source: String,
    /// `Key=effect`, repeatable.
    #[arg(long)]
    keyboard: Vec<String>,
    /// `boundary:against:minRatio:basis[:scope]`, repeatable. A REQUIREMENT,
    /// never a realisation (VDS S-2(6)).
    #[arg(long)]
    floor: Vec<String>,
    /// `name:type:true|false`, repeatable.
    #[arg(long)]
    prop: Vec<String>,
    #[arg(long)]
    supersedes: Vec<String>,
    #[arg(long)]
    basis: Vec<String>,
}

fn add(store: &Store, args: &AddArgs) -> Result<i32> {
    let status = parse_status(&args.status)?;
    if !matches!(status, Status::Proposed | Status::Designed) {
        return Err(VdsError::precondition(format!(
            "--status {status} would mint a record part-way along the lifecycle. VDS S-5(4) \
             makes the lifecycle a directed path where skipping is forbidden, and VDS S-6(2) \
             calls that ordering \"the entire mechanism\": every drift defect measured in the \
             motivating project was authored before anyone asked whether the thing being used \
             was registered.\n  \
             Add at `proposed` or `designed`, then advance with `vds register set-status`."
        )));
    }

    let code = match (&args.import_path, &args.source_file, &args.export_name) {
        (Some(import_path), Some(source_file), Some(export_name)) => {
            if source_file.starts_with('/') {
                return Err(VdsError::precondition(format!(
                    "--source-file {source_file:?} is absolute. Every record path is \
                     repository-relative, or the register stops meaning the same thing on \
                     another machine."
                )));
            }
            Some(CodeCounterpart {
                import_path: import_path.clone(),
                source_file: source_file.clone(),
                export_name: export_name.clone(),
            })
        }
        _ => None,
    };

    let id = ComponentId::allocate(&store.register_dir())?;
    let now = Timestamp::now();
    let mut record = ComponentRecord {
        id: id.clone(),
        name: args.name.clone(),
        status,
        contract_version: 1,
        figma: args.figma.as_deref().map(parse_figma).transpose()?,
        code,
        props: args
            .prop
            .iter()
            .map(|s| parse_prop(s))
            .collect::<Result<_>>()?,
        states: StateContract {
            required: parse_states(args.require.as_deref())?,
            drawn: parse_states(args.drawn.as_deref())?,
            built: parse_states(args.built.as_deref())?,
        }
        .normalised(),
        a11y: Accessibility {
            role: args.role.clone(),
            accessible_name_source: parse_name_source(&args.name_source)?,
            keyboard: args
                .keyboard
                .iter()
                .map(|s| parse_keyboard(s))
                .collect::<Result<_>>()?,
            contrast_floors: args
                .floor
                .iter()
                .map(|s| parse_floor(s))
                .collect::<Result<_>>()?,
        },
        demand: Demand {
            routes: 0,
            measured_at: now.clone(),
            measured_by: String::new(),
        },
        supersedes: args
            .supersedes
            .iter()
            .map(ComponentId::parse)
            .collect::<Result<_>>()?,
        superseded_by: None,
        amendments: vec![],
        basis: if args.basis.is_empty() {
            vec!["ACT-VDS-001:s5".to_owned()]
        } else {
            args.basis.clone()
        },
        deprecated_at: None,
        retired_at: None,
        retirement_proof_id: None,
        notes: None,
    };

    let (routes, measured_by) = measure_one(store, &record)?;
    record.demand = Demand {
        routes,
        measured_at: now,
        measured_by,
    };

    let path = store.record_path(&id);
    store.create(&path, &record)?;

    println!("registered {id} at {}", store.project.rel(&path));
    println!(
        "  demand measured at {routes} routes by: {}",
        record.demand.measured_by
    );
    if record.code.is_none() {
        println!(
            "  no code counterpart, so nothing can consume it yet. A record at `built` or \
             later must have one."
        );
    }
    Ok(PASSED)
}

// -------------------------------------------------------------- measure-demand

#[derive(ClapArgs)]
pub struct MeasureArgs {
    id: Option<String>,
    /// Re-measure every record.
    #[arg(long, conflicts_with = "id")]
    all: bool,
}

/// Count the routes on the declared surface that consume a component.
///
/// Measured, never estimated (VDS S-5(7)). The count comes from the generated
/// screens ledger, which is deterministic and re-runnable, and the command that
/// produced it travels with the number.
fn measure_one(store: &Store, record: &ComponentRecord) -> Result<(u32, String)> {
    let command = "vds register measure-demand".to_owned();
    let Some(code) = &record.code else {
        return Ok((
            0,
            format!("{command} (the record has no code counterpart, so nothing can consume it)"),
        ));
    };
    let ledger = vds_scan::load_fresh(store.project)?;
    let routes = ledger.routes_consuming(&code.import_path, &code.export_name);
    Ok((routes.len() as u32, command))
}

fn measure(store: &Store, args: &MeasureArgs) -> Result<i32> {
    let targets: Vec<Located<ComponentRecord>> = match (&args.id, args.all) {
        (Some(id), _) => vec![store.read_record(&ComponentId::parse(id)?)?],
        (None, true) => store.read_register()?,
        (None, false) => {
            return Err(VdsError::precondition(
                "name a component id, or pass --all. Measuring nothing in particular would \
                 leave the caller unsure which figures moved.",
            ));
        }
    };
    if targets.is_empty() {
        println!("the register is empty, so there is nothing to measure");
        return Ok(PASSED);
    }

    // Measure EVERYTHING first, then write. The retired tool wrote each record
    // as it went, so a failure part-way left some records re-measured and some
    // not, under a banner reading "VDS REFUSED, and did nothing".
    let now = Timestamp::now();
    let mut updated = Vec::with_capacity(targets.len());
    for located in targets {
        let (routes, measured_by) = measure_one(store, &located.value)?;
        let mut record = located.value;
        let before = record.demand.routes;
        record.demand = Demand {
            routes,
            measured_at: now.clone(),
            measured_by,
        };
        updated.push((located.path, record, before));
    }
    for (path, record, before) in &updated {
        store.replace(path, record)?;
        let moved = if *before == record.demand.routes {
            String::new()
        } else {
            format!("  (was {before})")
        };
        println!(
            "{}: demand.routes = {}{moved}",
            record.id, record.demand.routes
        );
    }
    Ok(PASSED)
}

// ---------------------------------------------------------------------- amend

#[derive(ClapArgs)]
pub struct AmendArgs {
    id: String,
    /// Declared kind. A declaration of `non_breaking` over a breaking change is
    /// refused: the classification is computed from the two records.
    #[arg(long, value_parser = ["non_breaking", "breaking"])]
    kind: String,
    #[arg(long)]
    what: String,
    #[arg(long)]
    by: Option<String>,
    #[arg(long)]
    warrant_id: Option<String>,
    #[arg(long)]
    proof_id: Option<String>,
    #[arg(long)]
    decision_log_id: Option<String>,
    #[arg(long)]
    add_required: Option<String>,
    #[arg(long)]
    remove_required: Option<String>,
    #[arg(long)]
    add_drawn: Option<String>,
    #[arg(long)]
    add_built: Option<String>,
    #[arg(long)]
    add_prop: Vec<String>,
    #[arg(long)]
    remove_prop: Vec<String>,
    #[arg(long)]
    set_floor: Vec<String>,
    #[arg(long)]
    role: Option<String>,
    #[arg(long)]
    name_source: Option<String>,
    #[arg(long, requires_all = ["source_file", "export_name"])]
    import_path: Option<String>,
    #[arg(long, requires_all = ["import_path", "export_name"])]
    source_file: Option<String>,
    #[arg(long, requires_all = ["import_path", "source_file"])]
    export_name: Option<String>,
    #[arg(long)]
    figma: Option<String>,
}

fn amend(store: &Store, args: &AmendArgs) -> Result<i32> {
    let id = ComponentId::parse(&args.id)?;
    let located = store.read_record(&id)?;
    let before = located.value.clone();
    let mut after = located.value;

    if let Some(states) = &args.add_required {
        after.states.required.extend(parse_states(Some(states))?);
    }
    if let Some(states) = &args.remove_required {
        let removing = parse_states(Some(states))?;
        after.states.required.retain(|s| !removing.contains(s));
    }
    if let Some(states) = &args.add_drawn {
        after.states.drawn.extend(parse_states(Some(states))?);
    }
    if let Some(states) = &args.add_built {
        after.states.built.extend(parse_states(Some(states))?);
    }
    after.states = after.states.normalised();

    for spec in &args.add_prop {
        let prop = parse_prop(spec)?;
        after.props.retain(|p| p.name != prop.name);
        after.props.push(prop);
    }
    for name in &args.remove_prop {
        after.props.retain(|p| &p.name != name);
    }
    after.props.sort_by(|a, b| a.name.cmp(&b.name));

    for spec in &args.set_floor {
        let floor = parse_floor(spec)?;
        after.a11y.contrast_floors.retain(|f| {
            (f.boundary.as_str(), f.against.as_str())
                != (floor.boundary.as_str(), floor.against.as_str())
        });
        after.a11y.contrast_floors.push(floor);
    }
    after.a11y.contrast_floors.sort_by(|a, b| {
        (a.boundary.as_str(), a.against.as_str()).cmp(&(b.boundary.as_str(), b.against.as_str()))
    });

    if let Some(role) = &args.role {
        after.a11y.role = Some(role.clone());
    }
    if let Some(source) = &args.name_source {
        after.a11y.accessible_name_source = parse_name_source(source)?;
    }
    if let (Some(import_path), Some(source_file), Some(export_name)) =
        (&args.import_path, &args.source_file, &args.export_name)
    {
        after.code = Some(CodeCounterpart {
            import_path: import_path.clone(),
            source_file: source_file.clone(),
            export_name: export_name.clone(),
        });
    }
    if let Some(figma) = &args.figma {
        after.figma = Some(parse_figma(figma)?);
    }

    if after == before {
        return Err(VdsError::precondition(
            "the amendment changes nothing, so there is nothing to record and no reason to \
             bump contractVersion. A version bump with no change makes the contract's history \
             unreadable.",
        ));
    }

    let reasons = breaking_reasons(&before, &after);
    let lowered: Vec<&str> = reasons
        .iter()
        .filter(|r| r.is_lowered_floor)
        .map(|r| r.what.as_str())
        .collect();

    if !reasons.is_empty() && args.kind == "non_breaking" {
        return Err(VdsError::precondition(format!(
            "this amendment is BREAKING under VDS S-9(4) and was declared non_breaking:\n  {}\n  \
             A breaking amendment requires a warrant, because the surface it invalidates is \
             the surface a warrant was granted over.",
            reasons
                .iter()
                .map(|r| r.what.as_str())
                .collect::<Vec<_>>()
                .join("\n  ")
        )));
    }

    // A warrant id is checked against the RECORD, not against its shape. The
    // retired tool accepted any string, so a contrast floor could be lowered
    // citing a warrant that does not exist.
    let warrant = match &args.warrant_id {
        Some(raw) => {
            let warrant_id = WarrantId::parse(raw)?;
            let warrant = store.read_warrant(&warrant_id)?;
            if warrant.value.status != WarrantStatus::Granted {
                return Err(VdsError::precondition(format!(
                    "{warrant_id} has status {} and cannot support an amendment. A warrant \
                     that was refused, spent, superseded or revoked authorises nothing.",
                    warrant.value.status
                )));
            }
            Some(warrant_id)
        }
        None => None,
    };

    if !lowered.is_empty() && warrant.is_none() {
        return Err(VdsError::precondition(format!(
            "refusing to lower a contrast floor without a granted warrant \
             (VDS S-9(4), S-9(5)):\n  {}\n  \
             Where a lower floor is genuinely correct, the lawful move is to change the \
             component's SCOPE and state the basis, not to loosen the ratio. A factual claim \
             about scope is contestable by a reviewer; a quietly lowered floor is not.",
            lowered.join("\n  ")
        )));
    }
    if args.kind == "breaking" && warrant.is_none() {
        return Err(VdsError::precondition(
            "a breaking amendment requires --warrant-id naming a GRANTED warrant (VDS S-9(4))",
        ));
    }

    after.contract_version = before.contract_version + 1;
    after.amendments.push(Amendment {
        at: Timestamp::now(),
        by: args.by.clone().unwrap_or_else(actor),
        kind: if args.kind == "breaking" {
            AmendmentKind::Breaking
        } else {
            AmendmentKind::NonBreaking
        },
        what: args.what.clone(),
        contract_version: after.contract_version,
        warrant_id: warrant,
        proof_id: args.proof_id.as_deref().map(ProofId::parse).transpose()?,
        decision_log_id: args.decision_log_id.clone(),
    });

    store.replace(&located.path, &after)?;
    println!(
        "amended {id} to contractVersion {} ({})",
        after.contract_version, args.kind
    );
    if !reasons.is_empty() {
        println!("  breaking because:");
        for reason in &reasons {
            println!("    {}", reason.what);
        }
    }
    if args.kind == "non_breaking" {
        println!(
            "  VDS S-9(3): a non-breaking amendment requires a decision log and a passing \
             reconciliation proof. Neither is written by this command."
        );
    }
    Ok(PASSED)
}

// ----------------------------------------------------------------- set-status

fn set_status(store: &Store, id: &str, target: &str) -> Result<i32> {
    let id = ComponentId::parse(id)?;
    let target = parse_status(target)?;
    let located = store.read_record(&id)?;
    let current = located.value.status;

    if matches!(target, Status::Deprecated | Status::Retired) {
        let command = if target == Status::Retired {
            "retire"
        } else {
            "deprecate"
        };
        return Err(VdsError::precondition(format!(
            "use `vds register {command}` for {target}. VDS S-9(6) makes retirement three \
             phases that cannot be compressed: supersession notice, drain to zero measured \
             demand, then tombstone. Assigning the status directly skips the drain, which is \
             the only phase that checks anything."
        )));
    }

    match current.ordinary_successor() {
        Some(next) if next == target => {}
        _ => {
            let path: Vec<&str> = Status::PATH.iter().map(|s| s.as_str()).collect();
            let advice = match current.ordinary_successor() {
                Some(next) => format!("The only lawful next status is {next}."),
                None => format!(
                    "{current} has no ordinary successor: the next transitions are deprecation \
                     and retirement, which are `vds register deprecate` and `vds register \
                     retire`."
                ),
            };
            return Err(VdsError::precondition(format!(
                "{id} is {current} and the lifecycle is a directed path where skipping is \
                 forbidden (VDS S-5(4)): {}.\n  {advice}",
                path.join(" -> ")
            )));
        }
    }

    let mut record = located.value;
    record.status = target;
    store.replace(&located.path, &record)?;
    println!("{id}: {current} -> {target}");
    Ok(PASSED)
}

// ------------------------------------------------------------------ deprecate

#[derive(ClapArgs)]
pub struct DeprecateArgs {
    id: String,
    /// The successor, which must itself be `registered` or later (VDS S-9(7)).
    #[arg(long, conflicts_with = "withdraw")]
    superseded_by: Option<String>,
    /// Withdrawn outright, with no replacement.
    #[arg(long)]
    withdraw: bool,
}

fn deprecate(store: &Store, args: &DeprecateArgs) -> Result<i32> {
    let id = ComponentId::parse(&args.id)?;
    let located = store.read_record(&id)?;
    if !located.value.status.is_enforceable() {
        return Err(VdsError::precondition(format!(
            "{id} is {} and only a registered component can be deprecated (VDS S-5(4)). A \
             component that was never registered is withdrawn by deleting nothing: there is \
             no consumer to notify.",
            located.value.status
        )));
    }

    let superseded_by = match (&args.superseded_by, args.withdraw) {
        (Some(successor_id), _) => {
            let successor_id = ComponentId::parse(successor_id)?;
            if successor_id == id {
                return Err(VdsError::precondition(
                    "a component cannot supersede itself",
                ));
            }
            let successor = store.read_record(&successor_id)?;
            if !successor.value.status.is_enforceable() {
                return Err(VdsError::precondition(format!(
                    "successor {successor_id} is {}. VDS S-9(7): the successor must itself be \
                     registered or later, because deprecating toward a component that does not \
                     yet exist is how a library ends up with two incomplete halves and no whole.",
                    successor.value.status
                )));
            }
            Some(successor_id)
        }
        (None, true) => None,
        (None, false) => {
            return Err(VdsError::precondition(
                "pass --superseded-by CMP-nnnn or --withdraw. Deprecating with neither leaves \
                 every consumer told to move and not told where.",
            ));
        }
    };

    let mut record = located.value;
    record.superseded_by = superseded_by.clone();
    record.status = Status::Deprecated;
    record.deprecated_at = Some(Timestamp::now());
    store.replace(&located.path, &record)?;

    let successor = superseded_by
        .map(|s| s.to_string())
        .unwrap_or_else(|| "nothing (withdrawn outright)".to_owned());
    println!("{id} deprecated, superseded by {successor}");
    println!(
        "  From now the composition proof reports every consuming site as a warning, per site, \
         by route. A deprecated component never passes silently (VDS S-9(6)(1))."
    );
    println!(
        "  Retirement needs a passing `retirement_drain` proof over zero measured demand \
         (VDS S-9(6)(2)). Run: vds proof retirement_drain"
    );
    Ok(PASSED)
}

// --------------------------------------------------------------------- retire

#[derive(ClapArgs)]
pub struct RetireArgs {
    id: String,
    /// The `retirement_drain` proof that MEASURED demand at zero.
    #[arg(long)]
    drain_proof: String,
}

fn retire(store: &Store, args: &RetireArgs) -> Result<i32> {
    let id = ComponentId::parse(&args.id)?;
    let located = store.read_record(&id)?;
    if located.value.status != Status::Deprecated {
        return Err(VdsError::precondition(format!(
            "{id} is {}. Retirement is three phases and cannot be compressed (VDS S-9(6)): \
             deprecate, drain to zero, tombstone.",
            located.value.status
        )));
    }

    let proof_id = ProofId::parse(&args.drain_proof)?;
    let proof = store.read_proof(&proof_id)?;
    if proof.value.kind != ProofKind::RetirementDrain {
        return Err(VdsError::precondition(format!(
            "{proof_id} is a {} proof, not retirement_drain. Only a drain proof measures \
             whether anything still consumes the component.",
            proof.value.kind
        )));
    }
    let defects = vds_proof::verify_record(&proof.value)?;
    if !defects.is_empty() {
        return Err(VdsError::precondition(format!(
            "{proof_id} cannot be cited:\n  {}",
            defects.join("\n  ")
        )));
    }

    if located.value.demand.routes != 0 {
        return Err(VdsError::precondition(format!(
            "{id} still has demand.routes = {}. VDS S-9(6)(2) and S-9(9) RESERVED \
             (SUBMISSION-VDS-004): the drain condition is absolute and no deadline overrides \
             a non-zero measured demand.\n  Re-measure with: vds register measure-demand {id}",
            located.value.demand.routes
        )));
    }

    let mut record = located.value;
    record.status = Status::Retired;
    record.retired_at = Some(Timestamp::now());
    record.retirement_proof_id = Some(proof_id.clone());
    store.replace(&located.path, &record)?;

    println!("{id} retired against {proof_id}.");
    println!(
        "  The record is kept forever and the identifier is never reused (VDS S-9(1), \
         S-9(6)(3))."
    );
    println!(
        "  From now the composition test INVERTS: a screen still referencing {id} is the \
         defect (VDS S-9(8))."
    );
    Ok(PASSED)
}

// ------------------------------------------------------------------- parsing

fn parse_status(raw: &str) -> Result<Status> {
    Status::parse(raw).ok_or_else(|| {
        VdsError::precondition(format!(
            "{raw:?} is not a lifecycle status. The seven are: {}",
            Status::PATH
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ))
    })
}

fn parse_states(raw: Option<&str>) -> Result<Vec<State>> {
    let Some(raw) = raw else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for item in raw.split(',') {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        let state = State::parse(item).ok_or_else(|| {
            VdsError::precondition(format!(
                "{item:?} is not one of the nine fixed states (VDS S-5(3)): {}",
                State::ALL
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        })?;
        if !out.contains(&state) {
            out.push(state);
        }
    }
    Ok(out)
}

fn parse_name_source(raw: &str) -> Result<NameSource> {
    Ok(match raw {
        "children" => NameSource::Children,
        "aria_label" => NameSource::AriaLabel,
        "aria_labelledby" => NameSource::AriaLabelledby,
        "title" => NameSource::Title,
        "alt" => NameSource::Alt,
        "none_decorative" => NameSource::NoneDecorative,
        other => {
            return Err(VdsError::precondition(format!(
                "{other:?} is not an accessible-name source. The six are: children, \
                 aria_label, aria_labelledby, title, alt, none_decorative"
            )));
        }
    })
}

fn parse_keyboard(spec: &str) -> Result<KeyboardContract> {
    let (key, effect) = spec
        .split_once('=')
        .ok_or_else(|| VdsError::precondition(format!("keyboard {spec:?} must be 'Key=effect'")))?;
    if key.trim().is_empty() || effect.trim().is_empty() {
        return Err(VdsError::precondition(format!(
            "keyboard {spec:?} must be 'Key=effect', and neither half may be empty"
        )));
    }
    Ok(KeyboardContract {
        key: key.trim().to_owned(),
        effect: effect.trim().to_owned(),
    })
}

fn parse_prop(spec: &str) -> Result<PropContract> {
    let parts: Vec<&str> = spec.split(':').collect();
    if parts.len() != 3 {
        return Err(VdsError::precondition(format!(
            "prop {spec:?} must be 'name:type:true|false'. The type must not contain a colon; \
             write a union as 'a|b|c'."
        )));
    }
    let required = match parts[2].trim() {
        "true" => true,
        "false" => false,
        other => {
            return Err(VdsError::precondition(format!(
                "prop {spec:?}: the third field is {other:?} and must be 'true' or 'false'"
            )));
        }
    };
    if parts[0].trim().is_empty() || parts[1].trim().is_empty() {
        return Err(VdsError::precondition(format!(
            "prop {spec:?}: neither the name nor the type may be empty"
        )));
    }
    Ok(PropContract {
        name: parts[0].trim().to_owned(),
        type_expr: parts[1].trim().to_owned(),
        required,
        figma_property: None,
    })
}

/// `boundary:against:minRatio:basis[:scope]`.
///
/// `minRatio` is a REQUIREMENT drawn from an external standard, not a
/// realisation (VDS S-2(6)). A malformed ratio is a precondition failure with a
/// sentence, not an unguarded parse that exits with the code reserved for
/// "violation".
fn parse_floor(spec: &str) -> Result<ContrastFloor> {
    let parts: Vec<&str> = spec.split(':').collect();
    if parts.len() != 4 && parts.len() != 5 {
        return Err(VdsError::precondition(format!(
            "floor {spec:?} must be 'boundary:against:minRatio:basis' with an optional \
             ':scope'. The basis must not contain a colon."
        )));
    }
    let min_ratio: f64 = parts[2].trim().parse().map_err(|_| {
        VdsError::precondition(format!(
            "floor {spec:?}: {:?} is not a ratio. It is a REQUIREMENT drawn from a standard, \
             for example 3.0 from WCAG 2.2 SC 1.4.11.",
            parts[2]
        ))
    })?;
    if !min_ratio.is_finite() || min_ratio < 1.0 {
        return Err(VdsError::precondition(format!(
            "floor {spec:?}: a contrast ratio is at least 1.0, which is a surface against \
             itself. {min_ratio} is not a ratio any standard states."
        )));
    }
    for (index, field) in [
        ("boundary", parts[0]),
        ("against", parts[1]),
        ("basis", parts[3]),
    ]
    .into_iter()
    .enumerate()
    {
        let _ = index;
        if field.1.trim().is_empty() {
            return Err(VdsError::precondition(format!(
                "floor {spec:?}: {} may not be empty. A floor with no basis is a number \
                 nobody can contest.",
                field.0
            )));
        }
    }
    let scope = match parts.get(4).map(|s| s.trim()) {
        None => None,
        Some("control_boundary") => Some(FloorScope::ControlBoundary),
        Some("text") => Some(FloorScope::Text),
        Some("graphical_object") => Some(FloorScope::GraphicalObject),
        Some("decoration") => Some(FloorScope::Decoration),
        Some(other) => {
            return Err(VdsError::precondition(format!(
                "floor scope {other:?} must be one of: control_boundary, text, \
                 graphical_object, decoration"
            )));
        }
    };
    Ok(ContrastFloor {
        boundary: parts[0].trim().to_owned(),
        against: parts[1].trim().to_owned(),
        min_ratio,
        basis: parts[3].trim().to_owned(),
        scope,
    })
}

/// `FILEKEY#12:34` or `FILEKEY#12-34`.
///
/// Both node-id spellings are accepted because both are what a designer
/// actually copies: `:` from a file URL, `-` from a deep link.
fn parse_figma(spec: &str) -> Result<FigmaNode> {
    let (file_key, node_id) = spec.split_once('#').ok_or_else(|| {
        VdsError::precondition(format!(
            "--figma {spec:?} must be 'FILEKEY#NODEID', for example 'aBc123#12:34'. Copy the \
             file key from the file URL and the node id from the selected layer."
        ))
    })?;
    if file_key.trim().is_empty() {
        return Err(VdsError::precondition(format!(
            "--figma {spec:?}: the file key is empty"
        )));
    }
    let node_id = node_id.trim();
    let well_formed = node_id.split_once([':', '-']).is_some_and(|(a, b)| {
        !a.is_empty()
            && !b.is_empty()
            && a.chars().all(|c| c.is_ascii_digit())
            && b.chars().all(|c| c.is_ascii_digit())
    });
    if !well_formed {
        return Err(VdsError::precondition(format!(
            "--figma {spec:?}: {node_id:?} is not a Figma node id. A node id is \
             <digits>:<digits> in a file URL, or <digits>-<digits> in a deep link, for \
             example '12:34'."
        )));
    }
    Ok(FigmaNode {
        file_key: file_key.trim().to_owned(),
        node_id: node_id.to_owned(),
        captured_at: Timestamp::now(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_floor_with_a_malformed_ratio_is_refused_with_a_sentence() {
        let error = parse_floor("text:bg:high:WCAG").unwrap_err();
        assert!(error.to_string().contains("is not a ratio"), "{error}");
    }

    #[test]
    fn a_floor_below_one_is_refused() {
        assert!(parse_floor("text:bg:0.5:WCAG").is_err());
        assert!(parse_floor("text:bg:1.0:WCAG").is_ok());
    }

    #[test]
    fn a_floor_needs_a_basis() {
        assert!(parse_floor("text:bg:3.0:").is_err());
    }

    #[test]
    fn a_floor_takes_an_optional_scope() {
        let floor = parse_floor("border:surface:3.0:WCAG 2.2 SC 1.4.11:control_boundary").unwrap();
        assert_eq!(floor.scope, Some(FloorScope::ControlBoundary));
        assert!(parse_floor("border:surface:3.0:WCAG:sparkling").is_err());
    }

    /// The retired tool's own `--figma` help said `FILEKEY#node:id`, and
    /// following it produced a node id the schema rejected with an unactionable
    /// message about oneOf branches.
    #[test]
    fn a_figma_node_id_is_checked_here_rather_than_by_a_schema_error() {
        assert!(parse_figma("KEY#node:id").is_err());
        assert!(parse_figma("KEY#12:34").is_ok());
        assert!(parse_figma("KEY#12-34").is_ok());
        assert!(parse_figma("KEY").is_err());
        assert!(parse_figma("#12:34").is_err());
        let error = parse_figma("KEY#node:id").unwrap_err();
        assert!(error.to_string().contains("<digits>:<digits>"), "{error}");
    }

    #[test]
    fn a_prop_needs_three_fields() {
        assert!(parse_prop("variant:string:true").is_ok());
        assert!(parse_prop("variant:string").is_err());
        assert!(parse_prop("variant:string:yes").is_err());
        assert!(parse_prop(":string:true").is_err());
    }

    #[test]
    fn a_tenth_state_is_refused_by_name() {
        let error = parse_states(Some("default,sparkling")).unwrap_err();
        assert!(error.to_string().contains("VDS S-5(3)"), "{error}");
    }

    #[test]
    fn states_are_deduplicated_and_empty_entries_ignored() {
        assert_eq!(
            parse_states(Some("default, ,default,hover")).unwrap(),
            vec![State::Default, State::Hover]
        );
    }
}
