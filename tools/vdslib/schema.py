"""A small JSON Schema validator, stdlib only.

VDS S-4(1): "a file that does not validate against its schema is not an artefact of
that kind". That sentence is only worth anything if something validates, so this
module implements the draft 2020-12 keywords the six VDS schemas actually use.

Supported: type, enum, const, required, properties, additionalProperties, items,
minItems, maxItems, uniqueItems, contains, minLength, maxLength, pattern, format
(date-time only, and only as a shape check), minimum, maximum, exclusiveMinimum,
exclusiveMaximum, multipleOf, allOf, anyOf, oneOf, not, if/then/else, $ref to a
local "#/..." pointer, and $defs.

A keyword this module does not implement is a HARD ERROR at load time, not a
silent skip. A validator that ignores what it cannot read is exactly the defect
VDS S-11(2) forbids in a designpack loader: silently lawless.
"""

from __future__ import annotations

import json
import re
from pathlib import Path

__all__ = ["Schema", "SchemaError", "ValidationError", "load_schema", "validate"]

_SUPPORTED = {
    "$schema",
    "$id",
    "$ref",
    "$defs",
    "$comment",
    "title",
    "description",
    "default",
    "examples",
    "deprecated",
    "type",
    "enum",
    "const",
    "required",
    "properties",
    "patternProperties",
    "additionalProperties",
    "propertyNames",
    "items",
    "prefixItems",
    "minItems",
    "maxItems",
    "uniqueItems",
    "contains",
    "minContains",
    "maxContains",
    "minLength",
    "maxLength",
    "pattern",
    "format",
    "minimum",
    "maximum",
    "exclusiveMinimum",
    "exclusiveMaximum",
    "multipleOf",
    "minProperties",
    "maxProperties",
    "allOf",
    "anyOf",
    "oneOf",
    "not",
    "if",
    "then",
    "else",
}

_DATE_TIME = re.compile(
    r"^[0-9]{4}-[0-9]{2}-[0-9]{2}[Tt][0-9]{2}:[0-9]{2}:[0-9]{2}(\.[0-9]+)?([Zz]|[+-][0-9]{2}:[0-9]{2})$"
)


class SchemaError(Exception):
    """The schema itself is outside what this validator implements."""


class ValidationError(Exception):
    """An instance failed validation. `errors` carries one line per failure."""

    def __init__(self, errors: list[str]) -> None:
        super().__init__("; ".join(errors))
        self.errors = errors


def _check_supported(node: object, where: str) -> None:
    if isinstance(node, dict):
        for key, value in node.items():
            if key in ("properties", "$defs", "patternProperties"):
                if isinstance(value, dict):
                    for sub_key, sub in value.items():
                        _check_supported(sub, f"{where}/{key}/{sub_key}")
                continue
            if key not in _SUPPORTED:
                raise SchemaError(f"{where}: unimplemented schema keyword {key!r}")
            _check_supported(value, f"{where}/{key}")
    elif isinstance(node, list):
        for i, sub in enumerate(node):
            _check_supported(sub, f"{where}[{i}]")


def _type_ok(value: object, name: str) -> bool:
    if name == "object":
        return isinstance(value, dict)
    if name == "array":
        return isinstance(value, list)
    if name == "string":
        return isinstance(value, str)
    if name == "boolean":
        return isinstance(value, bool)
    if name == "null":
        return value is None
    if name == "integer":
        return isinstance(value, int) and not isinstance(value, bool)
    if name == "number":
        return isinstance(value, (int, float)) and not isinstance(value, bool)
    raise SchemaError(f"unknown type name {name!r}")


class Schema:
    def __init__(self, root: dict, name: str = "<schema>") -> None:
        _check_supported(root, name)
        self.root = root
        self.name = name

    # -- pointer resolution ------------------------------------------------

    def _resolve(self, ref: str) -> dict:
        if not ref.startswith("#"):
            raise SchemaError(f"{self.name}: only local '#/...' refs are supported, got {ref!r}")
        node: object = self.root
        for part in ref.lstrip("#").strip("/").split("/"):
            if part == "":
                continue
            part = part.replace("~1", "/").replace("~0", "~")
            if not isinstance(node, dict) or part not in node:
                raise SchemaError(f"{self.name}: unresolvable ref {ref!r}")
            node = node[part]
        if not isinstance(node, dict):
            raise SchemaError(f"{self.name}: ref {ref!r} does not point at a schema object")
        return node

    # -- validation --------------------------------------------------------

    def errors_for(self, instance: object) -> list[str]:
        errors: list[str] = []
        self._validate(instance, self.root, "$", errors)
        return errors

    def validate(self, instance: object) -> None:
        errors = self.errors_for(instance)
        if errors:
            raise ValidationError(errors)

    def is_valid(self, instance: object) -> bool:
        return not self.errors_for(instance)

    def _validate(self, inst: object, sch: object, path: str, errors: list[str]) -> None:
        if sch is True:
            return
        if sch is False:
            errors.append(f"{path}: schema false, nothing is valid here")
            return
        if not isinstance(sch, dict):
            raise SchemaError(f"{path}: schema node must be an object or a boolean")

        if "$ref" in sch:
            self._validate(inst, self._resolve(sch["$ref"]), path, errors)

        if "type" in sch:
            names = sch["type"] if isinstance(sch["type"], list) else [sch["type"]]
            if not any(_type_ok(inst, n) for n in names):
                errors.append(f"{path}: expected type {'|'.join(names)}, got {_kind(inst)}")
                return

        if "enum" in sch and not any(_equal(inst, c) for c in sch["enum"]):
            errors.append(f"{path}: {inst!r} is not one of {sch['enum']!r}")
        if "const" in sch and not _equal(inst, sch["const"]):
            errors.append(f"{path}: expected constant {sch['const']!r}, got {inst!r}")

        if isinstance(inst, str):
            self._validate_string(inst, sch, path, errors)
        if isinstance(inst, (int, float)) and not isinstance(inst, bool):
            self._validate_number(inst, sch, path, errors)
        if isinstance(inst, list):
            self._validate_array(inst, sch, path, errors)
        if isinstance(inst, dict):
            self._validate_object(inst, sch, path, errors)

        for keyword in ("allOf",):
            for i, sub in enumerate(sch.get(keyword, [])):
                self._validate(inst, sub, path, errors)
        if "anyOf" in sch:
            if not any(not self._sub_errors(inst, sub, path) for sub in sch["anyOf"]):
                errors.append(f"{path}: matched none of the anyOf branches")
        if "oneOf" in sch:
            matched = [i for i, sub in enumerate(sch["oneOf"]) if not self._sub_errors(inst, sub, path)]
            if len(matched) != 1:
                errors.append(
                    f"{path}: expected exactly one oneOf branch to match, {len(matched)} did"
                )
        if "not" in sch and not self._sub_errors(inst, sch["not"], path):
            errors.append(f"{path}: matched a 'not' schema it must not match")
        if "if" in sch:
            if not self._sub_errors(inst, sch["if"], path):
                if "then" in sch:
                    self._validate(inst, sch["then"], path, errors)
            elif "else" in sch:
                self._validate(inst, sch["else"], path, errors)

    def _sub_errors(self, inst: object, sch: object, path: str) -> list[str]:
        errors: list[str] = []
        self._validate(inst, sch, path, errors)
        return errors

    def _validate_string(self, inst: str, sch: dict, path: str, errors: list[str]) -> None:
        if "minLength" in sch and len(inst) < sch["minLength"]:
            errors.append(f"{path}: shorter than minLength {sch['minLength']} ({len(inst)})")
        if "maxLength" in sch and len(inst) > sch["maxLength"]:
            errors.append(f"{path}: longer than maxLength {sch['maxLength']} ({len(inst)})")
        if "pattern" in sch and not re.search(sch["pattern"], inst):
            errors.append(f"{path}: {inst!r} does not match pattern {sch['pattern']!r}")
        fmt = sch.get("format")
        if fmt == "date-time" and not _DATE_TIME.match(inst):
            errors.append(f"{path}: {inst!r} is not an RFC 3339 date-time")

    def _validate_number(self, inst, sch: dict, path: str, errors: list[str]) -> None:
        if "minimum" in sch and inst < sch["minimum"]:
            errors.append(f"{path}: {inst} is below minimum {sch['minimum']}")
        if "maximum" in sch and inst > sch["maximum"]:
            errors.append(f"{path}: {inst} is above maximum {sch['maximum']}")
        if "exclusiveMinimum" in sch and inst <= sch["exclusiveMinimum"]:
            errors.append(f"{path}: {inst} is not above exclusiveMinimum {sch['exclusiveMinimum']}")
        if "exclusiveMaximum" in sch and inst >= sch["exclusiveMaximum"]:
            errors.append(f"{path}: {inst} is not below exclusiveMaximum {sch['exclusiveMaximum']}")
        if "multipleOf" in sch and sch["multipleOf"] and inst % sch["multipleOf"] != 0:
            errors.append(f"{path}: {inst} is not a multiple of {sch['multipleOf']}")

    def _validate_array(self, inst: list, sch: dict, path: str, errors: list[str]) -> None:
        if "minItems" in sch and len(inst) < sch["minItems"]:
            errors.append(f"{path}: has {len(inst)} items, minItems is {sch['minItems']}")
        if "maxItems" in sch and len(inst) > sch["maxItems"]:
            errors.append(f"{path}: has {len(inst)} items, maxItems is {sch['maxItems']}")
        if sch.get("uniqueItems") and _has_duplicates(inst):
            errors.append(f"{path}: items are not unique")
        offset = 0
        if "prefixItems" in sch:
            for i, sub in enumerate(sch["prefixItems"]):
                if i < len(inst):
                    self._validate(inst[i], sub, f"{path}[{i}]", errors)
            offset = len(sch["prefixItems"])
        if "items" in sch:
            for i in range(offset, len(inst)):
                self._validate(inst[i], sch["items"], f"{path}[{i}]", errors)
        if "contains" in sch:
            hits = sum(1 for i, v in enumerate(inst) if not self._sub_errors(v, sch["contains"], path))
            low = sch.get("minContains", 1)
            high = sch.get("maxContains")
            if hits < low:
                errors.append(f"{path}: {hits} items match 'contains', at least {low} required")
            if high is not None and hits > high:
                errors.append(f"{path}: {hits} items match 'contains', at most {high} allowed")

    def _validate_object(self, inst: dict, sch: dict, path: str, errors: list[str]) -> None:
        for key in sch.get("required", []):
            if key not in inst:
                errors.append(f"{path}: missing required property {key!r}")
        if "minProperties" in sch and len(inst) < sch["minProperties"]:
            errors.append(f"{path}: has {len(inst)} properties, minProperties is {sch['minProperties']}")
        if "maxProperties" in sch and len(inst) > sch["maxProperties"]:
            errors.append(f"{path}: has {len(inst)} properties, maxProperties is {sch['maxProperties']}")
        props = sch.get("properties", {})
        pattern_props = sch.get("patternProperties", {})
        for key, value in inst.items():
            handled = False
            if key in props:
                self._validate(value, props[key], f"{path}.{key}", errors)
                handled = True
            for pat, sub in pattern_props.items():
                if re.search(pat, key):
                    self._validate(value, sub, f"{path}.{key}", errors)
                    handled = True
            if "propertyNames" in sch:
                self._validate(key, sch["propertyNames"], f"{path}.<name:{key}>", errors)
            if not handled and "additionalProperties" in sch:
                extra = sch["additionalProperties"]
                if extra is False:
                    errors.append(f"{path}: property {key!r} is not permitted (additionalProperties false)")
                else:
                    self._validate(value, extra, f"{path}.{key}", errors)


def _kind(value: object) -> str:
    if value is None:
        return "null"
    if isinstance(value, bool):
        return "boolean"
    if isinstance(value, int):
        return "integer"
    if isinstance(value, float):
        return "number"
    if isinstance(value, str):
        return "string"
    if isinstance(value, list):
        return "array"
    if isinstance(value, dict):
        return "object"
    return type(value).__name__


def _equal(a: object, b: object) -> bool:
    if isinstance(a, bool) != isinstance(b, bool):
        return False
    if isinstance(a, (int, float)) and isinstance(b, (int, float)):
        return a == b
    return a == b


def _has_duplicates(items: list) -> bool:
    seen: list[str] = []
    for item in items:
        key = json.dumps(item, sort_keys=True, default=str)
        if key in seen:
            return True
        seen.append(key)
    return False


_CACHE: dict[str, Schema] = {}


def load_schema(schema_dir: Path, name: str) -> Schema:
    """Load `<schema_dir>/<name>.schema.json`, cached by resolved path."""
    path = Path(schema_dir) / f"{name}.schema.json"
    key = str(path.resolve())
    if key not in _CACHE:
        with open(path, "r", encoding="utf-8") as fh:
            _CACHE[key] = Schema(json.load(fh), name=name)
    return _CACHE[key]


def validate(schema_dir: Path, name: str, instance: object) -> list[str]:
    """Return a list of validation error lines. Empty means valid."""
    return load_schema(schema_dir, name).errors_for(instance)
