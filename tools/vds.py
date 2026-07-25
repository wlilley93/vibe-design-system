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
import re
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
    script_path_for,
    sha256_file,
    sha256_text,
)

STAGE_ORDER = ("W1", "W2", "W3", "W4")
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


ACCEPTANCE_PATH_RE = re.compile(r"^designpack/v[0-9]+/provenance/assent/[^/]+.*$")

# The one place a proof kind maps to the file that may produce it. A record is
# evidence only if it names THIS path for its kind, so a record cannot borrow a
# real script's digest under a kind that script does not implement.
def canonical_proof_script(kind: str) -> str:
    return f"tools/proofs/{kind}.py"


def proof_core_digest(record: dict) -> str:
    """Recompute the digest the capture path wrote over the proof's own result.

    Identical normalisation to vdslib.core.ProofRun._result_record. A record
    whose stated `digest` does not equal this has been edited after capture, so
    rows_enforced, status, violations and inputs_digest are all covered by one
    comparison rather than trusted as written.
    """
    violations = []
    for item in record.get("violations") or []:
        if isinstance(item, dict):
            violations.append(dict(item))
    core = {
        "kind": record.get("kind"),
        "status": record.get("status"),
        "rows_considered": record.get("rows_considered"),
        "rows_enforced": record.get("rows_enforced"),
        "rows_skipped_reasons": dict(sorted((record.get("rows_skipped_reasons") or {}).items())),
        "violations": sorted(
            violations,
            key=lambda v: (str(v.get("location", "")), str(v.get("rule", "")), str(v.get("actual", ""))),
        ),
        "inputs_digest": record.get("inputs_digest"),
    }
    return digest_of(core)


PROOF_DIGEST_LINE = re.compile(r"^digest:\s+(sha256:[0-9a-f]{64})\s*$", re.MULTILINE)


def reexecute_proof(project: Project, script_path: Path, record: dict, where: str) -> None:
    """Run the named check again and require the same digest (VDS S-7(2)(1)).

    The script binding says which bytes were meant to have run. It does not say
    they ran: a record naming the canonical script, its true digest and a
    self-consistent result digest can still be typed out by hand. The only thing
    that separates a record from an assertion is re-running the check and
    getting the same answer, so that is what happens here. The re-run writes
    nothing (--no-capture); it is a comparison, not a new claim.
    """
    command = [sys.executable, str(script_path), "--root", str(project.root), "--no-capture"]
    print(
        f"re-running {project.rel(script_path)} to confirm {where} reproduces ...",
        flush=True,
    )
    try:
        completed = subprocess.run(
            command, capture_output=True, text=True, timeout=1800, check=False
        )
    except OSError as exc:
        raise VdsError(
            f"{where} could not be re-run ({exc}). A proof record is citable only when the "
            "check behind it can be re-run and compared; if it cannot be run here, it "
            "cannot be relied on here."
        ) from exc

    found = PROOF_DIGEST_LINE.search(completed.stdout or "")
    if not found:
        detail = (completed.stderr or completed.stdout or "").strip().splitlines()
        tail = "\n    ".join(detail[-6:]) if detail else "(no output)"
        raise VdsError(
            f"{where} was re-run and the check did not produce a digest (exit "
            f"{completed.returncode}). It proves nothing until it runs cleanly:\n    {tail}"
        )
    if found.group(1) != record.get("digest"):
        raise VdsError(
            f"{where} DOES NOT REPRODUCE. The record states {record.get('digest')} and "
            f"re-running {project.rel(script_path)} over this tree right now yields "
            f"{found.group(1)}. Either the record was not produced by that run, or the "
            "tree has moved since. VDS S-7(2)(1): re-running the command must reproduce "
            "the same digest."
        )


def verify_proof_record(
    project: Project, proof_id: str, record: dict, reexecute: bool = False
) -> dict:
    """Refuse a proof record that is not bound to a real run of a real script.

    VDS S-7(2)(5) says a hand-written proof record is void. Fixing capture_mode
    to one enum value did not make that true: the field is a string an author
    types, so it asserted the property it was supposed to prove. Four things
    bind a record to an execution instead, in cost order:

      1. the kind must be one VDS actually implements;
      2. the record must name the canonical script for that kind, and that
         script must be on disk, so a record cannot borrow a digest it did not
         earn or name a script that never existed;
      3. the script's digest must still match the one recorded at capture, so a
         record whose check has since changed is stale rather than good;
      4. the record must digest to its own stated digest, so an edited
         rows_enforced, status or violations list is caught.

    With `reexecute`, the check is run again and required to reproduce the same
    digest. That is the only limb that distinguishes a record from an assertion,
    so `warrant record` sets it. `warrant status` is a report and does not.
    Returns the evidence entry to cite.
    """
    where = f"{proof_id} ({project.rel(Path(record.get('__path', proof_id)))})"

    kind = record.get("kind")
    if kind not in IMPLEMENTED_PROOF_KINDS:
        raise VdsError(
            f"{where} declares kind {kind!r}, which VDS specifies but does not implement. "
            f"Implemented kinds: {', '.join(IMPLEMENTED_PROOF_KINDS)}. A record for a "
            "script that does not exist is not evidence of anything, whatever it says "
            "about itself (VDS S-7(2)(5), S-7(5))."
        )

    # Validate the record as an artefact before reading anything else off it, so
    # a malformed forgery fails on its shape rather than on the first field that
    # happens to be checked.
    instance = {k: v for k, v in record.items() if not k.startswith("__")}
    project.validate_artefact("proof-result", instance, where)

    if record.get("status") != "passed":
        raise VdsError(
            f"{where} has status {record.get('status')!r}. A warrant may only cite a "
            "passed proof; a vacuous or failed proof is not evidence (VDS S-7(2)(4))."
        )
    if record.get("capture_mode") != "automatic":
        raise VdsError(
            f"{where} claims capture_mode {record.get('capture_mode')!r}. A "
            "hand-written proof record is void (VDS S-7(2)(5))."
        )

    expected_script = canonical_proof_script(str(kind))
    script = str(record.get("script", ""))
    if script != expected_script:
        raise VdsError(
            f"{where} names script {script!r}, but a {kind} record may only come from "
            f"{expected_script!r}. Citing another script's path is how a record borrows a "
            "digest it did not earn (VDS S-7(5): the registry of proof kinds is closed)."
        )
    script_path = script_path_for(project, script)
    if script_path is None:
        raise VdsError(
            f"{where} names {script!r}, which is not on disk under {project.root} or the "
            "VDS checkout. A proof record for a script that does not exist is refused as "
            "evidence (VDS S-7(2)(5))."
        )

    recorded_script_digest = record.get("script_digest")
    if not recorded_script_digest:
        raise VdsError(
            f"{where} carries no script_digest, so nothing ties it to an execution. "
            "Re-run the proof: records captured before script binding cannot be cited."
        )
    live_script_digest = sha256_file(script_path)
    if recorded_script_digest != live_script_digest:
        raise VdsError(
            f"{where} is STALE EVIDENCE: it was captured against {script} at "
            f"{recorded_script_digest}, and that script is now {live_script_digest}. The "
            "check that produced this record is not the check on disk. Re-run the proof."
        )

    recomputed = proof_core_digest(record)
    if record.get("digest") != recomputed:
        raise VdsError(
            f"{where} FAILS ITS OWN INTEGRITY CHECK: it states digest "
            f"{record.get('digest')} and its fields digest to {recomputed}. The record has "
            "been edited since capture (rows_enforced, status, violations or inputs_digest), "
            "so it is not the output of the run it claims (VDS S-7(2)(1))."
        )

    if int(record.get("rows_enforced") or 0) < 1:
        raise VdsError(
            f"{where} enforced 0 rows. A pass over zero enforceable rows is vacuous and is "
            "not evidence for any warrant (VDS S-7(2)(4))."
        )
    if int(record.get("exit_code", 1)) != 0:
        raise VdsError(
            f"{where} records exit_code {record.get('exit_code')!r} with status passed. "
            "The exit code is the contract a caller reads; the two must agree."
        )

    if reexecute:
        reexecute_proof(project, script_path, record, where)

    return {
        "proof_id": proof_id,
        "kind": kind,
        # Every field below is taken from the record on disk, never from the
        # caller: a warrant that cites a digest the caller supplied proves the
        # caller and nothing else.
        "digest": record.get("digest"),
        "script": script,
        "script_digest": live_script_digest,
        "status": "passed",
    }


def predecessor_chain(
    project: Project, stage: str, warrants: list[dict], live_surface: dict
) -> list[dict]:
    """Enforce W1 -> W2 -> W3 -> W4 (VDS S-6(2)).

    "A stage may not be entered before the preceding warrant is granted, and the
    ordering is the entire mechanism." Every earlier stage must hold a granted
    warrant, and it must have been granted over the surface that is live now: a
    predecessor granted over different bytes is spent (VDS S-6(4)), and building
    on a spent warrant is the ordering failing quietly rather than loudly.
    """
    chain = []
    for prior in STAGE_ORDER[: STAGE_ORDER.index(stage)]:
        held = [w for w in warrants if w.get("stage") == STAGE_NAMES[prior]]
        granted = [w for w in held if w.get("status") == "granted"]
        if not granted:
            seen = ", ".join(f"{w.get('id')}={w.get('status')}" for w in held) or "nothing on disk"
            raise VdsError(
                f"{stage} may not be entered: its predecessor {prior} "
                f"({STAGE_NAMES[prior]}) is not granted ({seen}). VDS S-6(2): a stage may "
                "not be entered before the preceding warrant is granted, and the ordering "
                f"is the entire mechanism. Record {prior} first, or record this one "
                "--status refused."
            )
        latest = granted[-1]
        surface = latest.get("surface") or {}
        if surface != live_surface:
            differing = [
                f"{key} granted-on {surface.get(key)} now {live_surface.get(key)}"
                for key in ("screens_digest", "register_digest")
                if surface.get(key) != live_surface.get(key)
            ]
            raise VdsError(
                f"{stage} may not be entered on {latest.get('id')}: that {prior} warrant was "
                f"granted over a different surface and is spent (VDS S-6(4)). "
                + "; ".join(differing)
                + f". Run: vds.py warrant spend {latest.get('id')}, then re-grant {prior}."
            )
        chain.append(
            {
                "warrant_id": str(latest.get("id")),
                "stage": STAGE_NAMES[prior],
                "digest": sha256_file(Path(latest["__path"])),
            }
        )
    return chain


ACCEPTANCE_TEMPLATE = """\
vds_acceptance_event: 1
project: {project}
stage: W3_PRINCIPAL_ACCEPTED
accepted_by: <name of the Principal, not "the Principal">
accepted_at: {now}
surface:
  screens_digest: {screens}
  register_digest: {register}
statement: <what was shown and what is accepted, in the acceptor's own words>
"""


def read_acceptance_event(project: Project, path: Path) -> dict:
    """Parse an acceptance event, as a whole file or as YAML front matter."""
    text = path.read_text(encoding="utf-8")
    if text.startswith("---\n"):
        _, _, rest = text.partition("---\n")
        block, marker, _ = rest.partition("\n---")
        if not marker:
            raise VdsError(
                f"{project.rel(path)} opens with '---' but never closes the front matter."
            )
        text = block
    try:
        data = yamlish.loads(text)
    except Exception as exc:
        raise VdsError(
            f"{project.rel(path)} is not a readable acceptance event: {exc}"
        ) from exc
    if not isinstance(data, dict):
        raise VdsError(f"{project.rel(path)} is not a mapping, so it declares nothing.")
    return data


def verify_acceptance_event(project: Project, raw_path: str, live_surface: dict) -> dict:
    """Refuse anything that is not an acceptance of THIS surface (VDS S-6(7)).

    A path regex is not a check: the auditor passed a one-line file that said in
    words it was not an assent event and it was accepted. An acceptance event
    must say what it is, for which project and stage, who accepted, when, and
    over exactly which surface digests, and the surface it names must be the
    surface on disk now. That last limb is the one that cannot be satisfied by
    accident: nothing lands in the file by chance that pins the live bytes.
    """
    event_path = Path(raw_path)
    if not event_path.is_file():
        raise VdsError(f"--acceptance-event {raw_path} does not exist")
    rel = project.rel(event_path)
    if not ACCEPTANCE_PATH_RE.match(rel):
        raise VdsError(
            f"--acceptance-event {rel} is not inside designpack/vN/provenance/assent/ of "
            f"{project.root}. The acceptance event lives in the record it accepts "
            "(VDS S-6(7))."
        )

    data = read_acceptance_event(project, event_path)
    template = ACCEPTANCE_TEMPLATE.format(
        project=project.jurisdiction_id,
        now=now_iso(),
        screens=live_surface["screens_digest"],
        register=live_surface["register_digest"],
    )

    def refuse(problem: str) -> None:
        raise VdsError(
            f"{rel} is not an acceptance event: {problem}\n"
            "An acceptance event is the whole of the W3 evidence, so it must be "
            "legible as one. Required shape (whole file, or YAML front matter):\n\n"
            + "\n".join("    " + line for line in template.splitlines())
        )

    if str(data.get("vds_acceptance_event", "")) != "1":
        refuse("it does not declare vds_acceptance_event: 1")
    if str(data.get("project", "")) != project.jurisdiction_id:
        refuse(
            f"it accepts for project {data.get('project')!r}, and this project is "
            f"{project.jurisdiction_id!r}"
        )
    if str(data.get("stage", "")) != STAGE_NAMES["W3"]:
        refuse(f"it names stage {data.get('stage')!r}, not {STAGE_NAMES['W3']}")
    accepted_by = str(data.get("accepted_by") or "").strip()
    if len(accepted_by) < 2 or accepted_by.lower() in ("the principal", "principal", "unknown"):
        refuse(
            f"accepted_by is {data.get('accepted_by')!r}. Name the person: acceptance is "
            "an act by someone, and 'the Principal' is a role VDS filled in by default"
        )
    accepted_at = str(data.get("accepted_at") or "").strip()
    if not re.match(r"^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$", accepted_at):
        refuse(f"accepted_at is {data.get('accepted_at')!r}, not a YYYY-MM-DDTHH:MM:SSZ instant")
    statement = str(data.get("statement") or "").strip()
    if len(statement) < 20:
        refuse("statement is missing or under 20 characters, so nothing was said")

    declared = data.get("surface")
    if not isinstance(declared, dict):
        refuse("it declares no surface: block, so it accepts nothing in particular")
    declared_surface = {
        "screens_digest": str(declared.get("screens_digest", "")),
        "register_digest": str(declared.get("register_digest", "")),
    }
    if declared_surface != live_surface:
        refuse(
            "the surface it accepts is not the surface on disk.\n"
            f"      accepted screens_digest:  {declared_surface['screens_digest']}\n"
            f"      live     screens_digest:  {live_surface['screens_digest']}\n"
            f"      accepted register_digest: {declared_surface['register_digest']}\n"
            f"      live     register_digest: {live_surface['register_digest']}\n"
            "      Acceptance is of specific bytes. If the surface moved, it was not "
            "accepted, and VDS may never infer acceptance from silence"
        )

    return {
        "path": rel,
        "digest": sha256_file(event_path),
        "accepted_at": accepted_at,
        "accepted_by": accepted_by,
        "surface_digest": digest_of(live_surface),
    }


def cmd_warrant(args) -> int:
    project = resolve(args)
    if args.action == "status":
        return _warrant_status(project)
    if args.action == "record":
        return _warrant_record(project, args)
    if args.action == "spend":
        return _warrant_spend(project, args)
    raise VdsError(f"unknown warrant action {args.action!r}")


def _report_chain(project: Project, warrant: dict, warrants: list[dict]) -> int:
    """Re-check a granted warrant's recorded ordering (VDS S-6(2), S-6(4))."""
    stage_name = str(warrant.get("stage", ""))
    stage = next((s for s, name in STAGE_NAMES.items() if name == stage_name), None)
    if stage is None:
        return 0
    expected = STAGE_ORDER[: STAGE_ORDER.index(stage)]
    chain = warrant.get("predecessors")
    if chain is None:
        if expected:
            print(
                f"     ORDERING UNRECORDED: this warrant names no predecessors, so nothing "
                f"shows {', '.join(expected)} was granted before it (VDS S-6(2)). It "
                "predates the ordering check; re-record it."
            )
            return 1
        return 0
    problems = 0
    if [str(row.get("stage")) for row in chain] != [STAGE_NAMES[s] for s in expected]:
        print(
            "     ORDERING BROKEN: predecessors are "
            f"{[row.get('stage') for row in chain]}, and {stage} requires "
            f"{[STAGE_NAMES[s] for s in expected]} (VDS S-6(2))."
        )
        problems += 1
    by_id = {str(w.get("id")): w for w in warrants}
    for row in chain:
        prior = by_id.get(str(row.get("warrant_id")))
        if prior is None:
            print(f"     PREDECESSOR MISSING: {row.get('warrant_id')} is not on disk")
            problems += 1
            continue
        live = sha256_file(Path(prior["__path"]))
        if live != row.get("digest"):
            print(
                f"     PREDECESSOR ALTERED: {row.get('warrant_id')} was "
                f"{row.get('digest')} when this warrant was recorded and is now {live}"
            )
            problems += 1
    return problems


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
                    else:
                        # Re-run the whole evidence gate, not just the digest
                        # equality: the script the record names may have changed
                        # or gone since, and a warrant standing on a proof that
                        # can no longer be shown to have run is a warrant on
                        # nothing (VDS S-7(2)(5)). This is a report, so it does
                        # NOT re-execute the checks; `warrant record` does that.
                        try:
                            verify_proof_record(
                                project, str(item.get("proof_id")), on_disk
                            )
                        except VdsError as exc:
                            print(f"     EVIDENCE NO LONGER STANDS: {exc}")
                            problems += 1
                acceptance = warrant.get("acceptance_event")
                if isinstance(acceptance, dict):
                    event = project.root / str(acceptance.get("path", ""))
                    if not event.is_file():
                        print(
                            f"     ACCEPTANCE EVENT MISSING: {acceptance.get('path')} is not "
                            "on disk, so nothing records that acceptance happened"
                        )
                        problems += 1
                    elif sha256_file(event) != acceptance.get("digest"):
                        print(
                            f"     ACCEPTANCE EVENT ALTERED: {acceptance.get('path')} was "
                            f"{acceptance.get('digest')} and is now {sha256_file(event)}"
                        )
                        problems += 1
                problems += _report_chain(project, warrant, warrants)
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
                # `warrant record` refuses a record that is not bound to a real run of a real
                # script. `warrant status` did not, so it happily printed a hand-written record
                # for an unimplemented kind as available evidence: the D2 defect surviving on
                # the surface D2 did not reach. A reader deciding whether to seek a warrant
                # reads THIS, so a false statement here is as misleading as a forged grant.
                # Same verifier, same refusal, one door.
                try:
                    verify_proof_record(project, proof.get("id", "?"), proof)
                    print(f"   evidence {kind}: {proof.get('id')} {proof.get('digest')}")
                except VdsError as exc:
                    # The refusal leads and the id follows in parentheses, deliberately. A
                    # reader skimming this column sees the verdict, not an id that looks like
                    # every valid one above it.
                    print(f"   evidence {kind}: NOT VALID EVIDENCE, and `warrant record` would "
                          f"refuse it (record {proof.get('id')}): {exc}")
                    problems += 1
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

    live_surface = surface_digests(project)
    warrants = read_warrants(project)

    # VDS S-6(2), the ordering, before anything else: a stage that may not be
    # entered has no business having its evidence weighed. A refusal is exempt,
    # because refusing a stage is a record that it was NOT granted.
    chain: list[dict] = []
    if args.status != "refused":
        chain = predecessor_chain(project, stage, warrants, live_surface)

    proofs = {p.get("id"): p for p in read_proofs(project)}
    evidence = []
    if stage == "W3" and args.evidence:
        raise VdsError(
            "W3 does not take --evidence. Its evidence is the acceptance event itself: "
            "acceptance is reserved to the Principal (VDS S-6(7)) and no proof, of any "
            "kind, substitutes for it. Passing a proof here is how a machine result gets "
            "dressed as a human decision."
        )
    for proof_id in args.evidence or []:
        proof = proofs.get(proof_id)
        if proof is None:
            raise VdsError(f"no proof record {proof_id!r} on disk")
        evidence.append(verify_proof_record(project, proof_id, proof, reexecute=True))
    have = {e["kind"] for e in evidence}
    # A refusal is a record that the stage was NOT granted, and the usual reason
    # to refuse is that this very evidence is missing. Requiring the full set
    # before a refusal may be written would make the honest record the one VDS
    # forbids. Whatever evidence IS cited on a refusal still has to be real.
    missing = [] if args.status == "refused" else [
        k for k in STAGE_EVIDENCE[stage] if k not in have
    ]
    if missing:
        unimplemented = [k for k in missing if k not in IMPLEMENTED_PROOF_KINDS]
        detail = ""
        if unimplemented:
            detail = (
                f"\n  Of those, {', '.join(unimplemented)} is specified and NOT IMPLEMENTED "
                "(vdslib/core.py IMPLEMENTED_PROOF_KINDS), so this stage cannot be granted "
                "by anyone until the script exists. That is the true state of the record, "
                "not a tooling gap to route around."
            )
        raise VdsError(
            f"{stage} requires evidence of kind {', '.join(STAGE_EVIDENCE[stage])} "
            f"(VDS S-6(2)); missing: {', '.join(missing)}." + detail
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

    acceptance = None
    if stage == "W3":
        if not args.acceptance_event:
            raise VdsError(
                "W3 needs --acceptance-event PATH. Acceptance is reserved to the Sovereign "
                "under ACT-001:s2, no proof substitutes for it, no bench may grant it, and "
                "VDS may never infer it from silence (VDS S-6(7))."
            )
        acceptance = verify_acceptance_event(project, args.acceptance_event, live_surface)
        for flag, field in (("--accepted-by", "accepted_by"), ("--accepted-at", "accepted_at")):
            given = getattr(args, field, None)
            if given and str(given) != acceptance[field]:
                raise VdsError(
                    f"{flag} says {given!r} and the acceptance event says "
                    f"{acceptance[field]!r}. The event is the record; a flag may not "
                    "contradict it."
                )
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
        # The ordering, written down. Checked at record time and re-checkable
        # afterwards: a predecessor edited later no longer digests to this.
        "predecessors": chain,
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
    print(
        "no enforcement drift: every pinned path matches its digest, every declared "
        "invoker was OPENED and names what it claims to run, every failing-direction "
        "test was OPENED and still contains its named test, and every claimed proof "
        "kind is one this tooling implements."
    )
    print(
        "  What this does NOT establish (VDS S-8(5)): that any invoker ever RAN. A "
        "workflow that exists, is readable and names the script, but is never "
        "triggered, passes every check above."
    )
    return EXIT_PASSED


def _lock_pin_name(project: Project, given: str, what: str) -> str:
    """Name a file the way a lock entry names it: repository-relative when it is
    in the project, `vds:`-prefixed when it lives in the VDS install instead.

    In a real adoption the proof scripts are in the install and not in the
    repository, so a project-relative-only name could not reach them at all.
    """
    if not locklib.is_safe_ref(given):
        raise VdsError(
            f"{what} {given!r} must be repository-relative (or "
            f"'{locklib.VDS_PATH_PREFIX}'-relative for a file inside the VDS install), "
            "with no leading slash and no '..'."
        )
    target = locklib.resolve_path(project, given)
    if target.is_file():
        return given
    if not given.startswith(locklib.VDS_PATH_PREFIX):
        in_install = locklib.VDS_HOME / given
        if in_install.is_file():
            return locklib.VDS_PATH_PREFIX + given
    raise VdsError(
        f"{what} {given!r} does not exist under {project.root} or under the VDS install "
        f"at {locklib.VDS_HOME}, so there is nothing there to name."
    )


def _lock_add(project: Project, args) -> int:
    if not args.invoked_by:
        raise VdsError(
            "pass --invoked-by at least once, as 'surface=ref' or 'surface=ref=blocking'. "
            "An empty invocation list is not representable, because an uninvoked gate is "
            "not enforcement (VDS S-7(2)(3))."
        )
    if not args.proves:
        raise VdsError(
            "pass --proves at least once. A script that produces no proof kind may not be "
            "pinned (VDS S-7(5))."
        )
    if not args.test_path or not args.test_name:
        raise VdsError(
            "pass --test-path and --test-name. An entry cannot be written without naming "
            "the test that proves the script's FAILING direction, which is how VDS "
            "S-7(2)(2) is made structural rather than aspirational."
        )

    pin_name = _lock_pin_name(project, args.path, "the path to pin")
    test_name_path = _lock_pin_name(project, args.test_path, "--test-path")
    target = locklib.resolve_path(project, pin_name)

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
    previous = next((e for e in entries if e.get("path") == pin_name), None)
    entries = [e for e in entries if e.get("path") != pin_name]

    entry = {
        "path": pin_name,
        "digest": sha256_file(target),
        "kind": args.kind,
        "invoked_by": invocations,
        "proves": list(args.proves or []),
        "failing_direction_test": {
            "path": test_name_path,
            "test_name": args.test_name,
        },
        "pinned_at": now_iso(),
        "pinned_by": actor(),
    }
    if args.seeds:
        entry["failing_direction_test"]["seeds"] = args.seeds

    # The door refuses exactly what the wall would find, from one implementation.
    # Writing an entry whose next `lock verify` is a guaranteed finding would put
    # a false claim of enforcement on disk, which is the thing the lock exists to
    # prevent rather than to hold.
    entry_findings, entry_notes = locklib.check_entry(project, entry)
    if entry_findings:
        raise VdsError(
            f"this entry would not survive `lock verify`, so it is not written. "
            f"{len(entry_findings)} findings, each named in full:\n  "
            + "\n  ".join(entry_findings)
        )

    if previous is not None and previous.get("digest") != entry["digest"]:
        if not args.rationale:
            raise VdsError(
                f"{pin_name} is already pinned at {previous.get('digest')} and the bytes "
                "have changed. Re-pinning is deliberate: pass --rationale, and self-file "
                "under VDS S-12(3). Re-locking without recording why is itself the breach "
                "the lock exists to make visible."
            )
        entry["supersedes_digest"] = previous.get("digest")
        entry["relock_rationale"] = args.rationale
    entries.append(entry)
    path = locklib.write(project, entries)
    print(f"pinned {pin_name} at {entry['digest']}")
    for invocation in entry["invoked_by"]:
        print(f"  invoker opened and confirmed: {invocation['ref']}")
    print(f"  failing-direction test opened and confirmed: {test_name_path}::{args.test_name}")
    for note in entry_notes:
        print(f"  {note}")
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
    l_add.add_argument(
        "path",
        help="repository-relative, or a path inside the VDS install (recorded as 'vds:...')",
    )
    l_add.add_argument(
        "--kind", default="proof_script", choices=["proof_script", "ledger_generator", "hook", "schema", "config"]
    )
    l_add.add_argument(
        "--invoked-by",
        action="append",
        help="'surface=ref[=blocking]'. The ref is OPENED and must name what it invokes.",
    )
    l_add.add_argument(
        "--proves",
        action="append",
        choices=list(PROOF_KINDS),
        help="only kinds this tooling implements may be pinned: "
        + ", ".join(IMPLEMENTED_PROOF_KINDS),
    )
    l_add.add_argument("--test-path", help="repository-relative, or 'vds:'-relative")
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
