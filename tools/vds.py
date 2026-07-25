#!/usr/bin/env python3
"""vds.py: the VDS front door.

VDS S-11(5): two front doors, exactly one wall. This CLI is a convenience door.
The wall is the proof scripts under tools/proofs/, which run whether or not this
door was used, and "the author used the tool" is never proof of conformance.

What this tool will NOT do, by design:

  - It does not GRANT a warrant. Granting is VJS's (VDS S-1(3), S-6(2)).
    `warrant record` writes down a grant that already happened, and refuses to
    write one that carries no grantor.
  - It does not decide anything contested. Every judgement call leaves by a
    submission under VDS S-10.
  - It does not read or write a design VALUE. Values live in the project's own
    systems of record (VDS S-2(2), S-2(3)).

Stdlib only. No install step.
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from vdslib import lock as locklib  # noqa: E402
from vdslib import scan, yamlish  # noqa: E402
from vdslib.core import (  # noqa: E402
    DEFAULT_CONFIG,
    EXIT_PASSED,
    EXIT_PRECONDITION,
    EXIT_VIOLATION,
    IMPLEMENTED_PROOF_KINDS,
    LIFECYCLE,
    NINE_STATES,
    PROOF_KINDS,
    Project,
    VdsError,
    actor,
    digest_of,
    find_project,
    now_iso,
    sha256_file,
    sha256_text,
)

STAGE_NUMBERS = {"W1": 1, "W2": 2, "W3": 3, "W4": 4}
STAGE_NAMES = {
    "W1": "W1_REGISTER_COMPLETE",
    "W2": "W2_DESIGN_COMPLETE",
    "W3": "W3_PRINCIPAL_ACCEPTED",
    "W4": "W4_PARITY",
}
STAGE_UNLOCKS = {
    "W1": ["design_may_begin"],
    "W2": ["principal_review"],
    "W3": ["parity_work_may_begin"],
    "W4": ["system_complete"],
}
STAGE_EVIDENCE = {
    "W1": ("register_completeness", "reconciliation"),
    "W2": ("composition", "states", "contrast"),
    "W3": (),
    "W4": ("parity", "token_pin", "contrast"),
}


# --------------------------------------------------------------------- helpers


def resolve(args) -> Project:
    return find_project(Path(args.root) if getattr(args, "root", None) else None)


def register_digest(project: Project) -> str:
    directory = project.path("register")
    rows = []
    if directory.is_dir():
        for file in sorted(directory.glob("*.yaml")):
            rows.append([project.rel(file), sha256_file(file)])
    return digest_of(rows)


def surface_digests(project: Project) -> dict:
    """The surface a warrant is granted over. A change to either digest spends it."""
    ledger_path = project.screens_ledger_path
    if ledger_path.is_file():
        ledger = yamlish.load(ledger_path)
        screens = str(ledger.get("source_digest", digest_of([])))
    else:
        screens = digest_of([])
    return {"screens_digest": screens, "register_digest": register_digest(project)}


def read_proofs(project: Project) -> list[dict]:
    directory = project.path("proofs")
    out = []
    if directory.is_dir():
        for file in sorted(directory.glob("*.yaml")):
            data = yamlish.load(file)
            if isinstance(data, dict):
                data["__path"] = str(file)
                out.append(data)
    return out


def latest_passed(proofs: list[dict], kind: str) -> dict | None:
    hits = [p for p in proofs if p.get("kind") == kind and p.get("status") == "passed"]
    return sorted(hits, key=lambda p: str(p.get("captured_at", "")))[-1] if hits else None


def read_warrants(project: Project) -> list[dict]:
    directory = project.path("warrants")
    out = []
    if directory.is_dir():
        for file in sorted(directory.glob("*.yaml")):
            data = yamlish.load(file)
            if isinstance(data, dict):
                data["__path"] = str(file)
                out.append(data)
    return out


def find_record(project: Project, component_id: str) -> tuple[Path, dict]:
    path = project.path("register") / f"{component_id}.yaml"
    if not path.is_file():
        raise VdsError(f"no register record at {project.rel(path)}")
    record = yamlish.load(path)
    if not isinstance(record, dict):
        raise VdsError(f"{project.rel(path)}: register record is not a mapping")
    return path, record


def parse_states(value: str | None) -> list[str]:
    if not value:
        return []
    out = []
    for item in value.split(","):
        item = item.strip()
        if not item:
            continue
        if item not in NINE_STATES:
            raise VdsError(
                f"{item!r} is not one of the nine fixed states (VDS S-5(3)): "
                f"{', '.join(NINE_STATES)}"
            )
        if item not in out:
            out.append(item)
    return out


def parse_floor(spec: str) -> dict:
    parts = spec.split(":")
    if len(parts) not in (4, 5):
        raise VdsError(
            f"floor {spec!r} must be 'boundary:against:minRatio:basis' with an optional "
            "':scope'. The basis must not contain a colon."
        )
    floor = {
        "boundary": parts[0].strip(),
        "against": parts[1].strip(),
        "minRatio": float(parts[2]),
        "basis": parts[3].strip(),
    }
    if len(parts) == 5:
        scope = parts[4].strip()
        allowed = ("control_boundary", "text", "graphical_object", "decoration")
        if scope not in allowed:
            raise VdsError(f"floor scope {scope!r} must be one of {', '.join(allowed)}")
        floor["scope"] = scope
    return floor


def parse_prop(spec: str) -> dict:
    head, _, required = spec.rpartition(":")
    if not head or required not in ("true", "false"):
        raise VdsError(f"prop {spec!r} must be 'name:type:true' or 'name:type:false'")
    name, _, type_expr = head.partition(":")
    if not name or not type_expr:
        raise VdsError(f"prop {spec!r} must be 'name:type:true|false'")
    return {
        "name": name.strip(),
        "type": type_expr.strip(),
        "required": required == "true",
        "figmaProperty": None,
    }


def parse_keyboard(spec: str) -> dict:
    key, _, effect = spec.partition("=")
    if not key.strip() or not effect.strip():
        raise VdsError(f"keyboard {spec!r} must be 'Key=effect'")
    return {"key": key.strip(), "effect": effect.strip()}


def measure_demand(project: Project, record: dict) -> tuple[int, str]:
    """Count the routes on the declared surface that consume this component.

    Measured, never estimated (VDS S-5(7)). The measurement is a count over the
    generated screens ledger, so it is deterministic and re-runnable.
    """
    code = record.get("code")
    command = "vds.py register measure-demand"
    if not isinstance(code, dict):
        return 0, command + " (record has no code counterpart, so nothing can consume it)"
    ledger = scan.load_fresh(project)
    import_path = str(code.get("importPath", ""))
    export_name = str(code.get("exportName", ""))
    routes = set()
    for screen in ledger.get("screens", []):
        for ref in screen.get("references", []):
            if ref.get("kind") != "component":
                continue
            if str(ref.get("importPath") or "") != import_path:
                continue
            if str(ref.get("name", "")).split(".")[0] != export_name:
                continue
            routes.add(screen.get("route"))
    return len(routes), command


# ------------------------------------------------------------------------ init


def cmd_init(args) -> int:
    root = Path(args.root or Path.cwd()).resolve()
    vds_dir = root / ".vds"
    config_path = vds_dir / "config.toml"
    if config_path.exists() and not args.force:
        print(f"{config_path} already exists. Pass --force to overwrite it.", file=sys.stderr)
        return EXIT_PRECONDITION

    for rel in (
        "register",
        "warrants",
        "proofs",
        "pins",
        "ledgers",
        "submissions/draft",
        "submissions/filed",
        "submissions/docket",
        "court/convenings",
        "logs/decisions",
        "logs/breaches",
        "permits",
    ):
        (vds_dir / rel).mkdir(parents=True, exist_ok=True)

    config_path.write_text(
        DEFAULT_CONFIG.format(
            jurisdiction_id=args.jurisdiction or root.name,
            repo_code=(args.repo_code or root.name).upper().replace("-", "_"),
        ),
        encoding="utf-8",
    )

    # VDS S-3(9): the record is committed, not scratch. Only cache/ and private/
    # are ignored, because a governance record that is gitignored is not a record.
    (vds_dir / ".gitignore").write_text("cache/\nprivate/\n", encoding="utf-8")

    designpack_dir = root / "designpack"
    if designpack_dir.is_dir():
        rows = [
            [str(p.relative_to(root)), sha256_file(p)]
            for p in sorted(designpack_dir.rglob("*"))
            if p.is_file()
        ]
        pack_digest = digest_of(rows)
        pack_id, pack_version = "local", "v1"
    else:
        pack_digest = sha256_text("vds:designpack:absent")
        pack_id, pack_version = "none", "0"

    yamlish.dump(
        {
            "schema_version": 1,
            "designpack_id": pack_id,
            "designpack_version": pack_version,
            "digest": pack_digest,
            "generated_at": now_iso(),
            "locked_by": actor(),
        },
        vds_dir / "designpack.lock",
    )

    print(f"initialised {vds_dir}")
    print("  config.toml, designpack.lock, .gitignore and the record directories")
    if pack_id == "none":
        print("")
        print("NOTE: no designpack/ is vendored, so designpack.lock pins the absence of one.")
        print("      VDS S-15(1): the specification commences on a dated, digest-pinned")
        print("      assent event in designpack/v1/provenance/assent/. Until then no")
        print("      warrant may be granted, because there is nothing to grant one under.")
    print("")
    print("Next: vds.py ledger screens, then vds.py proof --all")
    return EXIT_PASSED


# ---------------------------------------------------------------------- ledger


def cmd_ledger(args) -> int:
    project = resolve(args)
    if args.what != "screens":
        raise VdsError(f"unknown ledger {args.what!r}")
    path, ledger = scan.write(project)
    components = sum(
        1
        for s in ledger["screens"]
        for r in s["references"]
        if r["kind"] == "component"
    )
    elements = sum(
        1 for s in ledger["screens"] for r in s["references"] if r["kind"] == "element"
    )
    print(f"wrote {project.rel(path)}")
    print(f"  screens:              {len(ledger['screens'])}")
    print(f"  component references: {components}")
    print(f"  bare element references: {elements}  (informational only, VDS S-9(10) RESERVED)")
    print(f"  source_digest:        {ledger['source_digest']}")
    return EXIT_PASSED


# -------------------------------------------------------------------- register


def cmd_register(args) -> int:
    project = resolve(args)
    if args.action == "list":
        return _register_list(project)
    if args.action == "show":
        return _register_show(project, args)
    if args.action == "add":
        return _register_add(project, args)
    if args.action == "measure-demand":
        return _register_measure(project, args)
    if args.action == "amend":
        return _register_amend(project, args)
    if args.action == "set-status":
        return _register_set_status(project, args)
    if args.action == "deprecate":
        return _register_deprecate(project, args)
    if args.action == "retire":
        return _register_retire(project, args)
    raise VdsError(f"unknown register action {args.action!r}")


def _register_list(project: Project) -> int:
    records = project.read_register()
    if not records:
        print("the register is empty")
        return EXIT_PASSED
    width = max(len(str(r.get("name", ""))) for r in records)
    print(f"{len(records)} records in {project.rel(project.path('register'))}")
    for record in sorted(records, key=lambda r: str(r.get("id"))):
        code = record.get("code") or {}
        demand = record.get("demand") or {}
        print(
            f"  {record.get('id')}  {str(record.get('name','')):{width}}  "
            f"{str(record.get('status','')):11}  v{record.get('contractVersion')}  "
            f"routes={demand.get('routes')}  {code.get('importPath') or '(unbuilt)'}"
        )
    return EXIT_PASSED


def _register_show(project: Project, args) -> int:
    path, record = find_record(project, args.id)
    print(f"# {project.rel(path)}")
    print(yamlish.dumps(record), end="")
    return EXIT_PASSED


def _register_add(project: Project, args) -> int:
    component_id = project.next_component_id()
    code = None
    if args.import_path or args.source_file or args.export_name:
        if not (args.import_path and args.source_file and args.export_name):
            raise VdsError(
                "--import-path, --source-file and --export-name must be given together"
            )
        code = {
            "importPath": args.import_path,
            "sourceFile": args.source_file,
            "exportName": args.export_name,
        }
    figma = None
    if args.figma:
        file_key, _, node_id = args.figma.partition("#")
        if not file_key or not node_id:
            raise VdsError("--figma must be 'FILEKEY#node:id'")
        figma = {"fileKey": file_key, "nodeId": node_id, "capturedAt": now_iso()}

    record = {
        "id": component_id,
        "name": args.name,
        "status": args.status,
        "contractVersion": 1,
        "figma": figma,
        "code": code,
        "props": [parse_prop(p) for p in (args.prop or [])],
        "states": {
            "required": parse_states(args.require),
            "drawn": parse_states(args.drawn),
            "built": parse_states(args.built),
        },
        "a11y": {
            "role": args.role,
            "accessibleNameSource": args.name_source,
            "keyboard": [parse_keyboard(k) for k in (args.keyboard or [])],
            "contrastFloors": [parse_floor(f) for f in (args.floor or [])],
        },
        "demand": {"routes": 0, "measuredAt": now_iso(), "measuredBy": ""},
        "supersedes": list(args.supersedes or []),
        "supersededBy": None,
        "amendments": [],
        "basis": list(args.basis or ["ACT-VDS-001:s5"]),
    }
    routes, command = measure_demand(project, record)
    record["demand"] = {"routes": routes, "measuredAt": now_iso(), "measuredBy": command}

    path = project.path("register") / f"{component_id}.yaml"
    if path.exists():
        raise VdsError(
            f"{project.rel(path)} already exists. An identifier collision is a "
            "fail-closed validation error, never a silent overwrite (VDS S-4(4))."
        )
    project.write_artefact("component-record", path, record)
    print(f"registered {component_id} at {project.rel(path)}")
    print(f"  demand measured at {routes} routes by: {command}")
    if project.designpack_digest() == sha256_text("vds:designpack:absent"):
        print("")
        print("NOTE: no designpack is vendored, so this record's `basis` cites a statute")
        print("      section nothing can currently resolve (VDS S-10(4)).")
    return EXIT_PASSED


def _register_measure(project: Project, args) -> int:
    targets = project.read_register() if args.all else [find_record(project, args.id)[1]]
    if not targets:
        print("the register is empty, nothing to measure")
        return EXIT_PASSED
    for record in targets:
        record = dict(record)
        record.pop("__path", None)
        routes, command = measure_demand(project, record)
        record["demand"] = {"routes": routes, "measuredAt": now_iso(), "measuredBy": command}
        path = project.path("register") / f"{record['id']}.yaml"
        project.write_artefact("component-record", path, record)
        print(f"{record['id']}: demand.routes = {routes}   ({command})")
    return EXIT_PASSED


def _classify_amendment(before: dict, after: dict) -> list[str]:
    """Return the reasons this amendment is BREAKING, per VDS S-9(4)."""
    reasons: list[str] = []
    before_props = {p["name"]: p for p in before.get("props", [])}
    after_props = {p["name"]: p for p in after.get("props", [])}
    for name in sorted(set(before_props) - set(after_props)):
        reasons.append(f"prop {name!r} removed")
    for name in sorted(set(before_props) & set(after_props)):
        if not before_props[name]["required"] and after_props[name]["required"]:
            reasons.append(f"prop {name!r} became required")

    before_required = set(before.get("states", {}).get("required", []))
    after_required = set(after.get("states", {}).get("required", []))
    for state in sorted(before_required - after_required):
        reasons.append(f"required state {state!r} removed")

    before_a11y = before.get("a11y", {})
    after_a11y = after.get("a11y", {})
    if before_a11y.get("role") != after_a11y.get("role"):
        reasons.append(
            f"role changed from {before_a11y.get('role')!r} to {after_a11y.get('role')!r}"
        )
    if before_a11y.get("accessibleNameSource") != after_a11y.get("accessibleNameSource"):
        reasons.append(
            "accessible-name source changed from "
            f"{before_a11y.get('accessibleNameSource')!r} to "
            f"{after_a11y.get('accessibleNameSource')!r}"
        )

    before_floors = {
        (f["boundary"], f["against"]): f["minRatio"]
        for f in before_a11y.get("contrastFloors", [])
    }
    after_floors = {
        (f["boundary"], f["against"]): f["minRatio"]
        for f in after_a11y.get("contrastFloors", [])
    }
    for key in sorted(set(before_floors) - set(after_floors)):
        reasons.append(f"contrast floor {key[0]} against {key[1]} removed")
    for key in sorted(set(before_floors) & set(after_floors)):
        if after_floors[key] < before_floors[key]:
            reasons.append(
                f"contrast floor {key[0]} against {key[1]} LOWERED from "
                f"{before_floors[key]} to {after_floors[key]}"
            )
    return reasons


def _register_amend(project: Project, args) -> int:
    path, before = find_record(project, args.id)
    after = yamlish.loads(yamlish.dumps(before))

    if args.add_required:
        states = set(after["states"]["required"]) | set(parse_states(args.add_required))
        after["states"]["required"] = [s for s in NINE_STATES if s in states]
    if args.remove_required:
        states = set(after["states"]["required"]) - set(parse_states(args.remove_required))
        after["states"]["required"] = [s for s in NINE_STATES if s in states]
    if args.add_drawn:
        states = set(after["states"]["drawn"]) | set(parse_states(args.add_drawn))
        after["states"]["drawn"] = [s for s in NINE_STATES if s in states]
    if args.add_built:
        states = set(after["states"]["built"]) | set(parse_states(args.add_built))
        after["states"]["built"] = [s for s in NINE_STATES if s in states]
    for spec in args.add_prop or []:
        prop = parse_prop(spec)
        after["props"] = [p for p in after["props"] if p["name"] != prop["name"]] + [prop]
    for name in args.remove_prop or []:
        after["props"] = [p for p in after["props"] if p["name"] != name]
    for spec in args.set_floor or []:
        floor = parse_floor(spec)
        after["a11y"]["contrastFloors"] = [
            f
            for f in after["a11y"]["contrastFloors"]
            if (f["boundary"], f["against"]) != (floor["boundary"], floor["against"])
        ] + [floor]
    if args.role is not None:
        after["a11y"]["role"] = args.role
    if args.name_source is not None:
        after["a11y"]["accessibleNameSource"] = args.name_source
    if args.import_path or args.source_file or args.export_name:
        if not (args.import_path and args.source_file and args.export_name):
            raise VdsError(
                "--import-path, --source-file and --export-name must be given together"
            )
        after["code"] = {
            "importPath": args.import_path,
            "sourceFile": args.source_file,
            "exportName": args.export_name,
        }
    if args.figma:
        file_key, _, node_id = args.figma.partition("#")
        if not file_key or not node_id:
            raise VdsError("--figma must be 'FILEKEY#node:id'")
        after["figma"] = {"fileKey": file_key, "nodeId": node_id, "capturedAt": now_iso()}

    if yamlish.dumps(after) == yamlish.dumps(before):
        raise VdsError("the amendment changes nothing; refusing to bump contractVersion")

    reasons = _classify_amendment(before, after)
    lowered = [r for r in reasons if "LOWERED" in r]
    if reasons and args.kind == "non_breaking":
        raise VdsError(
            "this amendment is BREAKING under VDS S-9(4) and was declared non_breaking:\n  "
            + "\n  ".join(reasons)
            + "\n  A breaking amendment requires a warrant, because the surface it "
            "invalidates is the surface a warrant was granted over."
        )
    if lowered and not args.warrant_id:
        raise VdsError(
            "refusing to lower a contrast floor without a warrant (VDS S-9(4), S-9(5)):\n  "
            + "\n  ".join(lowered)
            + "\n  Where a lower floor is genuinely correct, the lawful move is to change "
            "the component's SCOPE and state the basis, not to loosen the ratio. A "
            "factual claim about scope is contestable by a reviewer; a quietly lowered "
            "floor is not."
        )
    if args.kind == "breaking" and not args.warrant_id:
        raise VdsError("a breaking amendment requires --warrant-id (VDS S-9(4))")

    after["contractVersion"] = int(before.get("contractVersion", 1)) + 1
    entry = {
        "at": now_iso(),
        "by": args.by or actor(),
        "kind": args.kind,
        "what": args.what,
        "contractVersion": after["contractVersion"],
    }
    if args.warrant_id:
        entry["warrantId"] = args.warrant_id
    if args.proof_id:
        entry["proofId"] = args.proof_id
    if args.decision_log_id:
        entry["decisionLogId"] = args.decision_log_id
    after.setdefault("amendments", []).append(entry)
    after.pop("__path", None)

    project.write_artefact("component-record", path, after)
    print(f"amended {args.id} to contractVersion {after['contractVersion']} ({args.kind})")
    if reasons:
        print("  breaking because:")
        for reason in reasons:
            print(f"    {reason}")
    if args.kind == "non_breaking":
        print(
            "  VDS S-9(3): a non-breaking amendment requires a decision log and a passing "
            "reconciliation proof. Neither is written by this command."
        )
    return EXIT_PASSED


def _register_set_status(project: Project, args) -> int:
    path, record = find_record(project, args.id)
    current = str(record.get("status"))
    target = args.status
    if target in ("deprecated", "retired"):
        raise VdsError(
            f"use `register {target[:-1] if target == 'retired' else 'deprecate'}` for "
            f"{target!r}: VDS S-9(6) makes retirement three phases that cannot be compressed"
        )
    if LIFECYCLE.index(target) != LIFECYCLE.index(current) + 1:
        raise VdsError(
            f"{args.id} is {current!r} and the lifecycle is a directed path where skipping "
            f"is forbidden (VDS S-5(4)): {' -> '.join(LIFECYCLE)}. "
            f"The only lawful next status is {LIFECYCLE[LIFECYCLE.index(current) + 1]!r}."
        )
    record["status"] = target
    record.pop("__path", None)
    project.write_artefact("component-record", path, record)
    print(f"{args.id}: {current} -> {target}")
    return EXIT_PASSED


def _register_deprecate(project: Project, args) -> int:
    path, record = find_record(project, args.id)
    if record.get("status") not in ("registered", "built", "verified"):
        raise VdsError(
            f"{args.id} is {record.get('status')!r}; only a registered component can be "
            "deprecated (VDS S-5(4))"
        )
    if args.superseded_by:
        _, successor = find_record(project, args.superseded_by)
        if successor.get("status") not in ("registered", "built", "verified"):
            raise VdsError(
                f"successor {args.superseded_by} is {successor.get('status')!r}. VDS S-9(7): "
                "the successor must itself be registered or later, because deprecating "
                "toward a component that does not yet exist is how a library ends up with "
                "two incomplete halves and no whole."
            )
        record["supersededBy"] = args.superseded_by
    elif args.withdraw:
        record["supersededBy"] = None
    else:
        raise VdsError("pass --superseded-by CMP-nnnn or --withdraw")
    record["status"] = "deprecated"
    record["deprecatedAt"] = now_iso()
    record.pop("__path", None)
    project.write_artefact("component-record", path, record)
    print(f"{args.id} deprecated, superseded by {record['supersededBy'] or 'nothing'}")
    print("  From now the composition proof reports every consuming site as a warning,")
    print("  per site, by route. A deprecated component never passes silently (VDS S-9(6)(1)).")
    return EXIT_PASSED


def _register_retire(project: Project, args) -> int:
    path, record = find_record(project, args.id)
    if record.get("status") != "deprecated":
        raise VdsError(
            f"{args.id} is {record.get('status')!r}. Retirement is three phases and cannot "
            "be compressed (VDS S-9(6)): deprecate, drain to zero, tombstone."
        )
    proofs = {p.get("id"): p for p in read_proofs(project)}
    proof = proofs.get(args.drain_proof)
    if proof is None:
        raise VdsError(
            f"no proof record {args.drain_proof!r} on disk. Retirement needs a "
            "retirement_drain proof that MEASURED demand at zero. That proof kind is "
            "specified (VDS S-7(5)) and is NOT implemented by this tooling, so retirement "
            "is currently unreachable, and that is the fail-closed state rather than a "
            "gap to route around."
        )
    if proof.get("kind") != "retirement_drain":
        raise VdsError(f"{args.drain_proof} is a {proof.get('kind')!r} proof, not retirement_drain")
    if proof.get("status") != "passed":
        raise VdsError(f"{args.drain_proof} has status {proof.get('status')!r}, not passed")
    routes = (record.get("demand") or {}).get("routes")
    if routes != 0:
        raise VdsError(
            f"{args.id} still has demand.routes = {routes}. VDS S-9(6)(2) and S-9(9) "
            "RESERVED (SUBMISSION-VDS-004): the drain condition is absolute and no "
            "deadline overrides a non-zero measured demand."
        )
    record["status"] = "retired"
    record["retiredAt"] = now_iso()
    record["retirementProofId"] = args.drain_proof
    record.pop("__path", None)
    project.write_artefact("component-record", path, record)
    print(f"{args.id} retired against {args.drain_proof}. The record is kept forever and the")
    print("identifier is never reused (VDS S-9(1), S-9(6)(3)).")
    return EXIT_PASSED


# ------------------------------------------------------------------------ proof

PROOF_MODULES = {
    "register_completeness": "proofs.register_completeness",
    "composition": "proofs.composition",
    "states": "proofs.states",
}


def cmd_proof(args) -> int:
    if args.list:
        print("proof kinds (VDS S-7(5), a CLOSED registry):")
        for kind in PROOF_KINDS:
            mark = "implemented" if kind in IMPLEMENTED_PROOF_KINDS else "NOT IMPLEMENTED"
            print(f"  {kind:22} {mark}")
        return EXIT_PASSED

    if args.all:
        kinds = list(IMPLEMENTED_PROOF_KINDS)
    elif args.kind:
        kinds = [args.kind]
    else:
        raise VdsError("name a proof kind, or pass --all, or pass --list")

    forwarded = []
    if args.root:
        forwarded += ["--root", args.root]
    if args.invoked_by:
        forwarded += ["--invoked-by", args.invoked_by]
    if args.allow_vacuous:
        forwarded += ["--allow-vacuous"]
    if args.no_capture:
        forwarded += ["--no-capture"]

    import importlib

    worst = EXIT_PASSED
    summary = []
    for kind in kinds:
        if kind not in PROOF_MODULES:
            raise VdsError(
                f"proof kind {kind!r} is in the closed registry (VDS S-7(5)) and is NOT "
                "implemented by this tooling. Adding a kind amends the specification and "
                "the invariant registry; it is not a script anyone may drop in (VDS S-7(6))."
            )
        print("=" * 72)
        module = importlib.import_module(PROOF_MODULES[kind])
        code = module.main() if not forwarded else _run_module(module, forwarded)
        summary.append((kind, code))
        worst = max(worst, code)
        print("")

    if len(kinds) > 1:
        print("=" * 72)
        print("summary:")
        for kind, code in summary:
            label = {0: "passed", 1: "FAILED", 2: "PRECONDITION FAILED", 3: "vacuous"}.get(
                code, f"exit {code}"
            )
            print(f"  {kind:24} {label}")
        missing = [k for k in PROOF_KINDS if k not in IMPLEMENTED_PROOF_KINDS]
        print("")
        print(
            f"{len(missing)} of the {len(PROOF_KINDS)} specified proof kinds are NOT "
            "implemented and did not run:"
        )
        print("  " + ", ".join(missing))
        print(
            "  A warrant relying on this run must not be described as covering them "
            "(VDS S-6(3))."
        )
    return worst


def _run_module(module, forwarded: list[str]) -> int:
    from vdslib.proofbase import guarded

    return guarded(lambda: module.run(forwarded))


# ---------------------------------------------------------------------- warrant


def cmd_warrant(args) -> int:
    project = resolve(args)
    if args.action == "status":
        return _warrant_status(project)
    if args.action == "record":
        return _warrant_record(project, args)
    if args.action == "spend":
        return _warrant_spend(project, args)
    raise VdsError(f"unknown warrant action {args.action!r}")


def _warrant_status(project: Project) -> int:
    warrants = read_warrants(project)
    proofs = read_proofs(project)
    live_surface = surface_digests(project)

    print("VDS grants nothing. Granting W1, W2 and W4 is VJS's on a referred submission,")
    print("and W3 is the Principal's alone (VDS S-1(3), S-6(2), S-6(7)). This is a report.")
    print("")
    print(f"live surface  screens_digest:  {live_surface['screens_digest']}")
    print(f"              register_digest: {live_surface['register_digest']}")
    print("")

    problems = 0
    for stage in ("W1", "W2", "W3", "W4"):
        stage_name = STAGE_NAMES[stage]
        held = [w for w in warrants if w.get("stage") == stage_name]
        granted = [w for w in held if w.get("status") == "granted"]
        print(f"{stage} {stage_name}")
        if not held:
            print("   status: NOT GRANTED, no warrant record exists")
            problems += 1
        for warrant in held:
            print(f"   {warrant.get('id')}  status: {warrant.get('status')}")
            if warrant.get("status") == "granted":
                surface = warrant.get("surface") or {}
                if surface != live_surface:
                    print(
                        "     SPENT: the surface has changed since this warrant was granted "
                        "(VDS S-6(4))."
                    )
                    for key in ("screens_digest", "register_digest"):
                        if surface.get(key) != live_surface.get(key):
                            print(f"       {key} granted-on: {surface.get(key)}")
                            print(f"       {key} now:        {live_surface.get(key)}")
                    print("     Record it: vds.py warrant spend " + str(warrant.get("id")))
                    problems += 1
                for item in warrant.get("evidence", []):
                    on_disk = next(
                        (p for p in proofs if p.get("id") == item.get("proof_id")), None
                    )
                    if on_disk is None:
                        print(f"     EVIDENCE MISSING: {item.get('proof_id')} is not on disk")
                        problems += 1
                    elif on_disk.get("digest") != item.get("digest"):
                        print(
                            f"     EVIDENCE DIGEST MISMATCH: {item.get('proof_id')} cites "
                            f"{item.get('digest')} and the record holds {on_disk.get('digest')}"
                        )
                        problems += 1
        required = STAGE_EVIDENCE[stage]
        for kind in required:
            proof = latest_passed(proofs, kind)
            if proof is None:
                note = (
                    "no passed proof on disk"
                    if kind in IMPLEMENTED_PROOF_KINDS
                    else "no passed proof on disk, and this kind is NOT IMPLEMENTED"
                )
                print(f"   evidence {kind}: {note}")
            else:
                print(f"   evidence {kind}: {proof.get('id')} {proof.get('digest')}")
        if stage == "W3":
            print("   evidence: an acceptance event, which no proof can substitute for")
        if not granted:
            print("   -> not granted")
        print("")

    print(
        f"{len(proofs)} proof records against "
        f"{len([w for w in warrants if w.get('status') == 'granted'])} granted warrants "
        "(docs/GOAL.md D9: the proof surface is the one that rots)."
    )
    return EXIT_VIOLATION if problems else EXIT_PASSED


def _warrant_record(project: Project, args) -> int:
    stage = args.stage
    if stage not in STAGE_NAMES:
        raise VdsError(f"--stage must be one of {', '.join(STAGE_NAMES)}")

    proofs = {p.get("id"): p for p in read_proofs(project)}
    evidence = []
    for proof_id in args.evidence or []:
        proof = proofs.get(proof_id)
        if proof is None:
            raise VdsError(f"no proof record {proof_id!r} on disk")
        if proof.get("status") != "passed":
            raise VdsError(
                f"{proof_id} has status {proof.get('status')!r}. A warrant may only cite a "
                "passed proof; a vacuous or failed proof is not evidence (VDS S-7(2)(4))."
            )
        if proof.get("capture_mode") != "automatic":
            raise VdsError(
                f"{proof_id} claims capture_mode {proof.get('capture_mode')!r}. A "
                "hand-written proof record is void (VDS S-7(2)(5))."
            )
        evidence.append(
            {
                "proof_id": proof_id,
                "kind": proof.get("kind"),
                # Taken from the record on disk, never from the caller: a warrant
                # that cites a digest the caller supplied proves the caller.
                "digest": proof.get("digest"),
                "status": "passed",
            }
        )
    have = {e["kind"] for e in evidence}
    missing = [k for k in STAGE_EVIDENCE[stage] if k not in have]
    if missing:
        raise VdsError(
            f"{stage} requires evidence of kind {', '.join(STAGE_EVIDENCE[stage])} "
            f"(VDS S-6(2)); missing: {', '.join(missing)}"
        )

    if args.case_file:
        case_path = Path(args.case_file)
        if not case_path.is_file():
            raise VdsError(f"--case-file {args.case_file} does not exist")
        case_file_digest = sha256_file(case_path)
    elif args.case_file_digest:
        case_file_digest = args.case_file_digest
    else:
        raise VdsError(
            "pass --case-file PATH or --case-file-digest. A warrant repeats the convening "
            "record's case_file_digest verbatim so what was decided on is provable after "
            "the fact (VDS S-10(5))."
        )

    live_surface = surface_digests(project)
    acceptance = None
    if stage == "W3":
        if not args.acceptance_event:
            raise VdsError(
                "W3 needs --acceptance-event PATH. Acceptance is reserved to the Sovereign "
                "under ACT-001:s2, no proof substitutes for it, no bench may grant it, and "
                "VDS may never infer it from silence (VDS S-6(7))."
            )
        event_path = Path(args.acceptance_event)
        if not event_path.is_file():
            raise VdsError(f"--acceptance-event {args.acceptance_event} does not exist")
        acceptance = {
            "path": project.rel(event_path),
            "digest": sha256_file(event_path),
            "accepted_at": args.accepted_at or now_iso(),
            "accepted_by": args.accepted_by or "the Principal",
            "surface_digest": digest_of(live_surface),
        }
        granted_by, assent_source = "principal", "principal_acceptance"
        bench: list[str] = []
        citation = None
    else:
        if not args.grantor_citation:
            raise VdsError(
                f"{stage} needs --grantor-citation, the VJS order that granted it. VDS "
                "grants nothing and may not grant itself a warrant (VDS S-1(3)). This "
                "command RECORDS a grant that already happened; it does not make one."
            )
        if not args.bench:
            raise VdsError(f"{stage} needs --bench, at least once: a warrant names its bench")
        granted_by, assent_source = "vjs_court", args.assent_source
        bench = list(args.bench)
        citation = args.grantor_citation

    warrant_id = project.next_warrant_id(STAGE_NUMBERS[stage])
    warrant = {
        "id": warrant_id,
        "stage": STAGE_NAMES[stage],
        "project": project.jurisdiction_id,
        "status": args.status,
        "issue": args.issue,
        "holding": args.holding,
        "granted_by": granted_by,
        "grantor_citation": citation,
        "assent_source": assent_source,
        "acceptance_event": acceptance,
        "evidence": evidence,
        "case_file_digest": case_file_digest,
        "directives": [],
        "forbidden": list(args.forbidden or []),
        "supersedes": list(args.supersedes or []),
        "unlocks": STAGE_UNLOCKS[stage],
        "surface": live_surface,
        "runtime_summary": args.runtime_summary,
        "created_at": now_iso(),
        "granted_at": args.granted_at or now_iso(),
        "bench": bench,
        "appealable": True,
        "reserved": list(args.reserved or []),
    }
    if stage in ("W1", "W2") and not warrant["reserved"]:
        # VDS S-9(10): any warrant relying on the informational-bare-elements
        # interim must say so. Composition and register_completeness both do.
        warrant["reserved"] = [
            "VDS S-9(10) RESERVED (SUBMISSION-VDS-005): the composition and "
            "register_completeness proofs treated bare HTML elements as informational "
            "rows, so this warrant does not reach the primitive layer."
        ]
    if stage == "W2":
        warrant["reserved"].append(
            "VDS S-6(6) RESERVED (SUBMISSION-VDS-002): who may grant W2 is unsettled. "
            "Until answered W2 is referred to VJS like W1 and W4, and a proof-only "
            "candidate may be recorded but never treated as granted."
        )

    path = project.path("warrants") / f"{warrant_id}.yaml"
    project.write_artefact("warrant", path, warrant)
    print(f"recorded {warrant_id} at {project.rel(path)}")
    print("")
    print("STANDING NOTE: recording is not granting. This file asserts that a grant")
    print("happened elsewhere and pins the evidence it was made on. If no such grant")
    print("happened, this record is a false statement of the record, not a warrant.")
    return EXIT_PASSED


def _warrant_spend(project: Project, args) -> int:
    path = project.path("warrants") / f"{args.id}.yaml"
    if not path.is_file():
        raise VdsError(f"no warrant at {project.rel(path)}")
    warrant = yamlish.load(path)
    if warrant.get("status") != "granted":
        raise VdsError(f"{args.id} has status {warrant.get('status')!r}, not granted")
    warrant["status"] = "spent"
    project.write_artefact("warrant", path, warrant)
    print(f"{args.id} marked spent. The record is never deleted (VDS S-6(4)).")
    return EXIT_PASSED


# ------------------------------------------------------------------------- lock


def cmd_lock(args) -> int:
    project = resolve(args)
    if args.action in ("verify", "status"):
        return _lock_verify(project)
    if args.action == "add":
        return _lock_add(project, args)
    if args.action == "repin":
        return _lock_repin(project, args)
    raise VdsError(f"unknown lock action {args.action!r}")


def _lock_verify(project: Project) -> int:
    findings, notes = locklib.verify(project)
    data = locklib.read(project)
    if data is not None:
        print(f"{len(data.get('entries', []))} entries in {locklib.LOCK_NAME}")
        for entry in data.get("entries", []):
            surfaces = ", ".join(
                f"{i.get('surface')}({'blocking' if i.get('blocking', True) else 'reporting'})"
                for i in entry.get("invoked_by", [])
            )
            test = entry.get("failing_direction_test", {})
            print(f"  {entry.get('path')}")
            print(f"    digest:  {entry.get('digest')}")
            print(f"    proves:  {', '.join(entry.get('proves', []))}")
            print(f"    invoked: {surfaces}")
            print(f"    failing-direction test: {test.get('path')}::{test.get('test_name')}")
            if test.get("seeds"):
                print(f"      seeds: {test.get('seeds')}")
    for note in notes:
        print("")
        print(note)
    if findings:
        print("")
        print(f"ENFORCEMENT DRIFT, {len(findings)} findings, each named in full:")
        for finding in findings:
            print(f"  {finding}")
        print("")
        print(
            "VDS S-8(5), stated plainly: the lock cannot bind an author with write access "
            "who edits a gate and re-locks it in the same act. It makes the act visible in "
            "a diff. It does not prevent it."
        )
        return EXIT_VIOLATION
    print("")
    print("no enforcement drift: every pinned path matches its digest.")
    return EXIT_PASSED


def _lock_add(project: Project, args) -> int:
    target = project.root / args.path
    if not target.is_file():
        raise VdsError(f"{args.path} does not exist, so there is nothing to pin")
    if not args.invoked_by:
        raise VdsError(
            "pass --invoked-by at least once, as 'surface=ref' or 'surface=ref=blocking'. "
            "An empty invocation list is not representable, because an uninvoked gate is "
            "not enforcement (VDS S-7(2)(3))."
        )
    if not args.test_path or not args.test_name:
        raise VdsError(
            "pass --test-path and --test-name. An entry cannot be written without naming "
            "the test that proves the script's FAILING direction, which is how VDS "
            "S-7(2)(2) is made structural rather than aspirational."
        )
    if not (project.root / args.test_path).is_file():
        raise VdsError(f"--test-path {args.test_path} does not exist")

    invocations = []
    for spec in args.invoked_by:
        parts = spec.split("=")
        if len(parts) not in (2, 3):
            raise VdsError(f"--invoked-by {spec!r} must be 'surface=ref' or 'surface=ref=blocking'")
        entry = {"surface": parts[0], "ref": parts[1]}
        if len(parts) == 3:
            entry["blocking"] = parts[2].lower() in ("1", "true", "yes", "blocking")
        invocations.append(entry)

    data = locklib.read(project)
    entries = list(data.get("entries", [])) if data else []
    previous = next((e for e in entries if e.get("path") == args.path), None)
    entries = [e for e in entries if e.get("path") != args.path]

    entry = {
        "path": args.path,
        "digest": sha256_file(target),
        "kind": args.kind,
        "invoked_by": invocations,
        "proves": list(args.proves or []),
        "failing_direction_test": {
            "path": args.test_path,
            "test_name": args.test_name,
        },
        "pinned_at": now_iso(),
        "pinned_by": actor(),
    }
    if args.seeds:
        entry["failing_direction_test"]["seeds"] = args.seeds
    if previous is not None and previous.get("digest") != entry["digest"]:
        if not args.rationale:
            raise VdsError(
                f"{args.path} is already pinned at {previous.get('digest')} and the bytes "
                "have changed. Re-pinning is deliberate: pass --rationale, and self-file "
                "under VDS S-12(3). Re-locking without recording why is itself the breach "
                "the lock exists to make visible."
            )
        entry["supersedes_digest"] = previous.get("digest")
        entry["relock_rationale"] = args.rationale
    entries.append(entry)
    path = locklib.write(project, entries)
    print(f"pinned {args.path} at {entry['digest']}")
    print(f"  wrote {project.rel(path)} ({len(entries)} entries)")
    return EXIT_PASSED


def _lock_repin(project: Project, args) -> int:
    data = locklib.read(project)
    if data is None:
        raise VdsError(f"no {locklib.LOCK_NAME} to re-pin")
    if not args.rationale:
        raise VdsError(
            "re-pinning needs --rationale. Re-locking without recording why, and without "
            "self-filing under VDS S-12(3), is itself the breach the lock exists to make "
            "visible (VDS S-8(4))."
        )
    entries = []
    changed = []
    for entry in data.get("entries", []):
        actual = locklib.current_digest(project, entry["path"])
        if actual is None:
            raise VdsError(
                f"{entry['path']} is pinned and missing. Re-pinning a deleted gate would "
                "erase the finding rather than answer it."
            )
        if actual != entry["digest"]:
            changed.append((entry["path"], entry["digest"], actual))
            entry = dict(entry)
            entry["supersedes_digest"] = entry["digest"]
            entry["relock_rationale"] = args.rationale
            entry["digest"] = actual
            entry["pinned_at"] = now_iso()
            entry["pinned_by"] = actor()
        entries.append(entry)
    if not changed:
        print("nothing to re-pin: every pinned path already matches its digest")
        return EXIT_PASSED
    locklib.write(project, entries)
    print(f"re-pinned {len(changed)} entries, each recording what it superseded:")
    for path, old, new in changed:
        print(f"  {path}")
        print(f"    was: {old}")
        print(f"    now: {new}")
    print(f"  rationale: {args.rationale}")
    return EXIT_PASSED


# ------------------------------------------------------------------------- main


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="vds.py", description=__doc__.split("\n")[2])
    parser.add_argument("--root", default=None, help="project root holding .vds/config.toml")
    sub = parser.add_subparsers(dest="command", required=True)

    p_init = sub.add_parser("init", help="scaffold a project's .vds/")
    p_init.add_argument("--jurisdiction", default=None)
    p_init.add_argument("--repo-code", default=None)
    p_init.add_argument("--force", action="store_true")
    p_init.set_defaults(func=cmd_init)

    p_ledger = sub.add_parser("ledger", help="regenerate a generated inventory")
    p_ledger.add_argument("what", choices=["screens"])
    p_ledger.set_defaults(func=cmd_ledger)

    p_reg = sub.add_parser("register", help="add, amend or retire a component record")
    reg_sub = p_reg.add_subparsers(dest="action", required=True)
    p_reg.set_defaults(func=cmd_register)

    r_list = reg_sub.add_parser("list")
    r_list.set_defaults(action="list")
    r_show = reg_sub.add_parser("show")
    r_show.add_argument("id")
    r_show.set_defaults(action="show")

    r_add = reg_sub.add_parser("add")
    r_add.add_argument("--name", required=True)
    r_add.add_argument("--status", default="proposed", choices=list(LIFECYCLE))
    r_add.add_argument("--import-path")
    r_add.add_argument("--source-file")
    r_add.add_argument("--export-name")
    r_add.add_argument("--figma", help="FILEKEY#node:id")
    r_add.add_argument("--require", help="comma separated states that are REQUIRED")
    r_add.add_argument("--drawn", help="comma separated states already DRAWN")
    r_add.add_argument("--built", help="comma separated states already BUILT")
    r_add.add_argument("--role", default=None)
    r_add.add_argument(
        "--name-source",
        default="children",
        choices=["children", "aria_label", "aria_labelledby", "title", "alt", "none_decorative"],
    )
    r_add.add_argument("--keyboard", action="append", help="'Key=effect', repeatable")
    r_add.add_argument(
        "--floor",
        action="append",
        help="'boundary:against:minRatio:basis[:scope]', repeatable. A REQUIREMENT, "
        "never a realisation (VDS S-2(6)).",
    )
    r_add.add_argument("--prop", action="append", help="'name:type:true|false', repeatable")
    r_add.add_argument("--supersedes", action="append")
    r_add.add_argument("--basis", action="append")
    r_add.set_defaults(action="add")

    r_measure = reg_sub.add_parser("measure-demand")
    r_measure.add_argument("id", nargs="?")
    r_measure.add_argument("--all", action="store_true")
    r_measure.set_defaults(action="measure-demand")

    r_amend = reg_sub.add_parser("amend")
    r_amend.add_argument("id")
    r_amend.add_argument("--kind", required=True, choices=["non_breaking", "breaking"])
    r_amend.add_argument("--what", required=True)
    r_amend.add_argument("--by", default=None)
    r_amend.add_argument("--warrant-id", default=None)
    r_amend.add_argument("--proof-id", default=None)
    r_amend.add_argument("--decision-log-id", default=None)
    r_amend.add_argument("--add-required")
    r_amend.add_argument("--remove-required")
    r_amend.add_argument("--add-drawn")
    r_amend.add_argument("--add-built")
    r_amend.add_argument("--add-prop", action="append")
    r_amend.add_argument("--remove-prop", action="append")
    r_amend.add_argument("--set-floor", action="append")
    r_amend.add_argument("--role", default=None)
    r_amend.add_argument(
        "--name-source",
        default=None,
        choices=["children", "aria_label", "aria_labelledby", "title", "alt", "none_decorative"],
    )
    r_amend.add_argument("--import-path")
    r_amend.add_argument("--source-file")
    r_amend.add_argument("--export-name")
    r_amend.add_argument("--figma")
    r_amend.set_defaults(action="amend")

    r_status = reg_sub.add_parser("set-status")
    r_status.add_argument("id")
    r_status.add_argument("status", choices=list(LIFECYCLE))
    r_status.set_defaults(action="set-status")

    r_dep = reg_sub.add_parser("deprecate")
    r_dep.add_argument("id")
    r_dep.add_argument("--superseded-by")
    r_dep.add_argument("--withdraw", action="store_true")
    r_dep.set_defaults(action="deprecate")

    r_ret = reg_sub.add_parser("retire")
    r_ret.add_argument("id")
    r_ret.add_argument("--drain-proof", required=True)
    r_ret.set_defaults(action="retire")

    p_proof = sub.add_parser("proof", help="run one or all proofs")
    p_proof.add_argument("kind", nargs="?", choices=list(PROOF_KINDS))
    p_proof.add_argument("--all", action="store_true")
    p_proof.add_argument("--list", action="store_true")
    p_proof.add_argument(
        "--invoked-by",
        default=None,
        choices=[
            "githook_pre_commit",
            "githook_pre_push",
            "ci_workflow",
            "package_script",
            "build",
            "manual",
        ],
    )
    p_proof.add_argument("--allow-vacuous", action="store_true")
    p_proof.add_argument("--no-capture", action="store_true")
    p_proof.set_defaults(func=cmd_proof)

    p_war = sub.add_parser("warrant", help="report warrant status, or record a granted warrant")
    war_sub = p_war.add_subparsers(dest="action", required=True)
    p_war.set_defaults(func=cmd_warrant)

    w_status = war_sub.add_parser("status")
    w_status.set_defaults(action="status")

    w_record = war_sub.add_parser(
        "record", help="write down a grant that already happened; this does NOT grant"
    )
    w_record.add_argument("--stage", required=True, choices=list(STAGE_NAMES))
    w_record.add_argument("--issue", required=True)
    w_record.add_argument("--holding", required=True)
    w_record.add_argument("--runtime-summary", required=True)
    w_record.add_argument("--evidence", action="append")
    w_record.add_argument("--grantor-citation", default=None)
    w_record.add_argument("--bench", action="append")
    w_record.add_argument(
        "--assent-source",
        default="sovereign_assent",
        choices=["sovereign_assent", "standing_bounded_assent", "principal_acceptance"],
    )
    w_record.add_argument("--acceptance-event", default=None)
    w_record.add_argument("--accepted-by", default=None)
    w_record.add_argument("--accepted-at", default=None)
    w_record.add_argument("--case-file", default=None)
    w_record.add_argument("--case-file-digest", default=None)
    w_record.add_argument("--granted-at", default=None)
    w_record.add_argument(
        "--status", default="granted", choices=["granted", "refused", "spent", "superseded", "revoked"]
    )
    w_record.add_argument("--forbidden", action="append")
    w_record.add_argument("--supersedes", action="append")
    w_record.add_argument("--reserved", action="append")
    w_record.set_defaults(action="record")

    w_spend = war_sub.add_parser("spend")
    w_spend.add_argument("id")
    w_spend.set_defaults(action="spend")

    p_lock = sub.add_parser("lock", help="recompute and diff enforcement.lock")
    lock_sub = p_lock.add_subparsers(dest="action", required=True)
    p_lock.set_defaults(func=cmd_lock)

    l_verify = lock_sub.add_parser("verify")
    l_verify.set_defaults(action="verify")

    l_add = lock_sub.add_parser("add")
    l_add.add_argument("path")
    l_add.add_argument(
        "--kind", default="proof_script", choices=["proof_script", "ledger_generator", "hook", "schema", "config"]
    )
    l_add.add_argument("--invoked-by", action="append", help="'surface=ref[=blocking]'")
    l_add.add_argument("--proves", action="append", choices=list(PROOF_KINDS))
    l_add.add_argument("--test-path")
    l_add.add_argument("--test-name")
    l_add.add_argument("--seeds")
    l_add.add_argument("--rationale")
    l_add.set_defaults(action="add")

    l_repin = lock_sub.add_parser("repin")
    l_repin.add_argument("--rationale")
    l_repin.set_defaults(action="repin")

    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    try:
        return args.func(args)
    except VdsError as exc:
        print("VDS REFUSED, and did nothing:", file=sys.stderr)
        print(f"  {exc}", file=sys.stderr)
        return EXIT_PRECONDITION


if __name__ == "__main__":
    try:
        sys.exit(main())
    except KeyboardInterrupt:
        sys.exit(EXIT_PRECONDITION)
