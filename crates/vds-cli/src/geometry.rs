//! `vds geometry`: declare a bound on how many surfaces of one SHAPE do not
//! comply, and lower it.
//!
//! The front door to the tenth artefact kind. It is a convenience door and not
//! the wall: `geometry` runs whether or not this was used, and "the author used
//! the tool" is never proof of conformance (VDS S-11(5)).
//!
//! # Why the verb is `lower` and not `set`
//!
//! This is the whole of VDS S-7A(2) expressed as an interface. A `set` verb
//! makes raising a bound exactly as easy as lowering it, one character apart,
//! and the instrument S-7A(2) was enacted against was a ratchet whose number went
//! 667 to 561 and then stopped for good. A command named `lower` that refuses to
//! raise puts the direction in the reader's hands before the proof has to say
//! anything, and the route to a genuine re-baseline is a NEW record with the
//! reason on it: a bound that goes up is not the same bound.
//!
//! # What this command does NOT decide
//!
//! What counts as a compliant radius, boundary weight, spacing step or type step.
//! That is the subject's design system talking, and VDS holding those thresholds
//! would make it a fourth design authority. VDS holds the bound and the
//! direction; the subject's generator holds the meaning.
//!
//! This command reads and writes no design value (VDS S-2(2)). A COUNT of
//! non-compliant surfaces is a fact about conformance; a radius is the design's
//! own answer and has nowhere to live.

use clap::{Args as ClapArgs, Subcommand};
use vds_core::{
    BoundEntry, EXIT_VIOLATION, GeometryBound, GeometryId, Result, Status, SurfaceKind, Timestamp,
    VdsError,
};
use vds_store::Store;

use crate::{Context, PASSED};

#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand)]
enum Action {
    /// Declare a BASELINE bound for one surface kind.
    Add(AddArgs),
    /// Lower an existing bound. There is deliberately no way to raise one.
    Lower(LowerArgs),
    /// Every bound, what is in force, and whether it is falling.
    List,
}

#[derive(ClapArgs)]
pub struct AddArgs {
    /// Which shape: radius, boundary_weight, density or type_scale.
    ///
    /// One record per kind, per VDS S-7A(3). A single number for the whole
    /// estate names no work: it cannot be assigned, it cannot be finished, and
    /// it hides which shapes are worst.
    #[arg(long)]
    kind: String,
    /// How many non-compliant surfaces of this kind are admitted from now.
    ///
    /// The BASELINE, so it is normally today's measured count. Declaring it
    /// higher than the count buys slack that the direction rule will take back
    /// at the first window, and declaring it at or above the whole population
    /// makes a bound nothing can exceed, which the proof refuses outright.
    #[arg(long)]
    bound: u32,
    /// How many days the bound may stand without falling.
    ///
    /// Declared by the project, because the rate a backlog can be worked down is
    /// a fact about the subject and not about VDS. What VDS fixes is that the
    /// window exists and that expiry is fatal.
    #[arg(long, default_value_t = 30)]
    window_days: u32,
    /// Why this baseline, in one line.
    #[arg(long)]
    because: Option<String>,
    #[arg(long, default_value = "registered")]
    status: String,
    #[arg(long, value_delimiter = ',', default_value = "ACT-VDS-001:s7a")]
    basis: Vec<String>,
}

#[derive(ClapArgs)]
pub struct LowerArgs {
    /// The bound to lower.
    #[arg(long)]
    id: String,
    /// The new, LOWER bound.
    #[arg(long)]
    to: u32,
    /// What lowered it. Printed in the finding when a bound later stops moving,
    /// because "declare a lower bound" is advice and "the last three reductions
    /// came from these three pieces of work" is a plan.
    #[arg(long)]
    because: String,
}

pub fn run(ctx: &Context, args: &Args) -> Result<i32> {
    let project = ctx.project()?;
    let store = Store::new(&project);
    match &args.action {
        Action::Add(a) => add(&store, a),
        Action::Lower(a) => lower(&store, a),
        Action::List => list(&store),
    }
}

fn parse_kind(raw: &str) -> Result<SurfaceKind> {
    SurfaceKind::parse(raw).ok_or_else(|| {
        VdsError::precondition(format!(
            "{raw:?} is not one of the four surface kinds (VDS S-7A(3)): {}.\n  The set is \
             closed. A shape outside it is an amendment to the specification, not a new \
             string here, because an open set of surface kinds is how one undifferentiated \
             bucket comes back.",
            SurfaceKind::ALL
                .iter()
                .map(|k| k.as_str())
                .collect::<Vec<&str>>()
                .join(", ")
        ))
    })
}

fn add(store: &Store, args: &AddArgs) -> Result<i32> {
    let kind = parse_kind(&args.kind)?;
    let status = Status::parse(&args.status).ok_or_else(|| {
        VdsError::precondition(format!(
            "{:?} is not one of the seven lifecycle statuses (VDS S-5(4)): {}",
            args.status,
            Status::PATH
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<&str>>()
                .join(", ")
        ))
    })?;

    // Refused at the DOOR as well as at the wall, and the asymmetry is
    // deliberate. Two enforceable bounds for one kind is R8 in the proof, but
    // discovering it there means the second record is already on disk and the
    // author has to be told to undo something. Here they are told before they
    // do it, and the refusal names the lawful route.
    let existing = store.read_geometry()?;
    if let Some(clash) = existing
        .iter()
        .find(|r| r.value.surface_kind == kind && r.value.status.is_enforceable())
        && status.is_enforceable()
    {
        return Err(VdsError::precondition(format!(
            "{} already holds an enforceable bound for {kind}, and two bounds for one shape \
             means nothing says which governs (VDS S-7A(3), geometry R8).\n  To move this \
             bound, `vds geometry lower --id {} --to <n> --because <why>`, which keeps the \
             history that is the only evidence the count ever fell.\n  To re-baseline after \
             a genuine population change, DEPRECATE the existing record first. Do not delete \
             it: a bound with no history cannot be shown to be falling.",
            clash.value.id, clash.value.id
        )));
    }

    let id = GeometryId::allocate(&store.geometry_dir())?;
    let record = GeometryBound {
        id: id.clone(),
        surface_kind: kind,
        status,
        declared_window_days: args.window_days,
        history: vec![BoundEntry {
            at: Timestamp::now(),
            bound: args.bound,
            because: args.because.clone(),
        }],
        basis: args.basis.clone(),
        notes: None,
    };
    let path = store.geometry_path(&id);
    store.create(&path, &record)?;

    println!("registered {id} [{kind}]");
    println!("  path:    {}", store.project.rel(&path));
    println!("  status:  {}", record.status);
    println!(
        "  bound:   {} non-compliant surfaces admitted (a COUNT; a radius is a realisation \
         and has no field here, VDS S-2(4))",
        args.bound
    );
    println!("  window:  {} days", args.window_days);
    println!();
    println!(
        "This is a BASELINE and not a reduction. VDS S-7A(2) requires the bound to FALL, and \
         a first declaration is not a fall, so `vds proof geometry` will report this record \
         as not-yet-falling until `vds geometry lower` is run against it. That is correct: \
         registering a number is not progress on it."
    );
    Ok(PASSED)
}

fn lower(store: &Store, args: &LowerArgs) -> Result<i32> {
    let id = GeometryId::parse(&args.id)?;
    let path = store.geometry_path(&id);
    if !path.is_file() {
        return Err(VdsError::precondition(format!(
            "no geometry bound at {}",
            store.project.rel(&path)
        )));
    }
    let mut record: GeometryBound = store.read(&path)?;

    let Some(current) = record.current().cloned() else {
        return Err(VdsError::precondition(format!(
            "{id} has an empty history, so there is no bound to lower. Its baseline was never \
             declared."
        )));
    };

    if args.to >= current.bound {
        return Err(VdsError::precondition(format!(
            "{id} is at {} and --to is {}, which is not lower.\n  There is no `set` verb here \
             and no way to raise a bound, and that is VDS S-7A(2) expressed as an interface \
             rather than left to the proof: the instrument this clause was enacted against \
             was a ratchet that could only be HELD, whose number moved 667 to 561 through \
             work done for other reasons and then stopped for good.\n  If the population \
             genuinely grew, the honest record is a NEW baseline with the reason on it, \
             raised after deprecating this one. A bound that goes up is not the same bound.",
            current.bound, args.to
        )));
    }

    record.history.push(BoundEntry {
        at: Timestamp::now(),
        bound: args.to,
        because: Some(args.because.clone()),
    });
    store.replace(&path, &record)?;

    println!(
        "{id} [{}] lowered {} -> {}",
        record.surface_kind, current.bound, args.to
    );
    println!("  because: {}", args.because);
    println!(
        "  next:    the window is {} days from now (VDS S-7A(2))",
        record.declared_window_days
    );
    Ok(PASSED)
}

fn list(store: &Store) -> Result<i32> {
    let records = store.read_geometry()?;
    if records.is_empty() {
        println!("no geometry bound is declared.");
        println!(
            "  `geometry` is the only proof kind that reads a surface's SHAPE, and with no \
             bound declared its runs are VACUOUS and prove nothing (VDS S-7(2)(4)). Every \
             other kind reads a NAME: which token, which component, which arrangement. \
             Nothing else in the registry can see that a product looks unchanged."
        );
        return Ok(PASSED);
    }

    // Read alongside the bounds, so `list` can say whether each one is actually
    // falling rather than only what it says. A listing that printed the numbers
    // and left the reader to work out the direction would be the ratchet's own
    // presentation.
    let reading = vds_core::read_reading(store.project)?;

    println!("{} geometry bound(s):", records.len());
    let mut stalled = 0;
    for located in &records {
        let record = &located.value;
        let current = record
            .current()
            .map_or("none".to_owned(), |e| e.bound.to_string());
        let measured = reading
            .as_ref()
            .and_then(|r| r.kind(record.surface_kind))
            .map_or("not measured".to_owned(), |k| {
                if k.undecided == 0 {
                    format!("{} of {}", k.non_compliant, k.considered)
                } else {
                    format!(
                        "{} to {} of {}",
                        k.non_compliant,
                        k.worst_case(),
                        k.considered
                    )
                }
            });
        let direction = match record.last_reduction() {
            Some(r) => format!("last fell {} to {}", &r.at.as_str()[..10], r.bound),
            None => {
                stalled += 1;
                "NEVER FALLEN".to_owned()
            }
        };
        println!(
            "  {:10} {:16} {:10} bound {:>6}   measured {:14}   {}",
            record.id.as_str(),
            record.surface_kind.to_string(),
            record.status.as_str(),
            current,
            measured,
            direction
        );
    }

    let kinds_covered: Vec<SurfaceKind> = records
        .iter()
        .filter(|r| r.value.status.is_enforceable())
        .map(|r| r.value.surface_kind)
        .collect();
    let uncovered: Vec<&str> = SurfaceKind::ALL
        .iter()
        .filter(|k| !kinds_covered.contains(k))
        .map(|k| k.as_str())
        .collect();
    if !uncovered.is_empty() {
        println!();
        println!(
            "{} of the four shapes have no enforceable bound: {}. Those shapes are governed \
             by nothing, and the proof cannot report what it was never asked to look at.",
            uncovered.len(),
            uncovered.join(", ")
        );
    }
    if reading.is_none() {
        println!();
        println!(
            "NO READING EXISTS, so every bound above is compared against nothing. `vds proof \
             geometry` reports each as UNKNOWN rather than met (VDS S-7A(4)): a bound with no \
             measurement behind it is a promise, and this listing is not evidence that it is \
             being kept."
        );
        return Ok(EXIT_VIOLATION);
    }
    if stalled > 0 {
        println!();
        println!(
            "{stalled} of them have NEVER fallen. VDS S-7A(2): a bound that may only be held \
             is a floor, and a floor is a different instrument from a target."
        );
        return Ok(EXIT_VIOLATION);
    }
    Ok(PASSED)
}
