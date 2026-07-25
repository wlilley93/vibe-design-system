"""A restricted YAML subset reader and writer, stdlib only.

VDS artefacts are `.yaml` files (VDS S-4(1)). VDS tooling takes no third-party
dependency, so this module implements exactly the subset the artefact schemas can
express, and refuses anything outside it rather than guessing.

Supported on load:
  - block mappings            key: value
  - block sequences           - item
  - nesting by indentation (spaces only, tabs are rejected)
  - plain, single-quoted and double-quoted scalars
  - flow collections written as JSON               [] {} ["a", "b"]
  - literal and folded block scalars               | and >
  - comments, from an unquoted '#' to end of line
  - null (null, ~, empty), true/false, integers, floats

Deliberately NOT supported, and rejected loudly: anchors and aliases, tags,
multiple documents, complex keys, tabs for indentation. A parser that silently
skips what it cannot read is the same defect class VDS S-11(2) forbids in a
designpack loader.

JSON is a subset of YAML 1.2, so a file written by `dump` is readable by any
conformant YAML reader, and a JSON file is readable by `load` here.
"""

from __future__ import annotations

import json
import re

__all__ = ["load", "loads", "dump", "dumps", "YamlishError"]


class YamlishError(ValueError):
    """Raised when input falls outside the supported subset."""


_INT_RE = re.compile(r"^[+-]?[0-9]+$")
_FLOAT_RE = re.compile(r"^[+-]?(?:[0-9]+\.[0-9]*|\.[0-9]+|[0-9]+)(?:[eE][+-]?[0-9]+)?$")


# --------------------------------------------------------------------------- load


class _Line:
    __slots__ = ("no", "indent", "text")

    def __init__(self, no: int, indent: int, text: str) -> None:
        self.no = no
        self.indent = indent
        self.text = text

    def __repr__(self) -> str:  # pragma: no cover - debugging aid
        return f"_Line(no={self.no}, indent={self.indent}, text={self.text!r})"


def _strip_comment(raw: str) -> str:
    out = []
    quote = None
    i = 0
    while i < len(raw):
        ch = raw[i]
        if quote is None:
            if ch in ("'", '"'):
                quote = ch
            elif ch == "#" and (i == 0 or raw[i - 1] in " \t"):
                break
        else:
            if ch == "\\" and quote == '"':
                out.append(ch)
                i += 1
                if i < len(raw):
                    out.append(raw[i])
                    i += 1
                continue
            if ch == quote:
                quote = None
        out.append(ch)
        i += 1
    return "".join(out).rstrip()


def _tokenise(text: str) -> list[_Line]:
    lines: list[_Line] = []
    for no, raw in enumerate(text.splitlines(), start=1):
        if "\t" in raw[: len(raw) - len(raw.lstrip(" \t"))]:
            raise YamlishError(f"line {no}: tab used for indentation, which is not YAML")
        stripped_full = raw.strip()
        if stripped_full in ("---", "..."):
            continue
        body = _strip_comment(raw)
        if not body.strip():
            continue
        indent = len(body) - len(body.lstrip(" "))
        lines.append(_Line(no, indent, body.strip()))
    return lines


def _scalar(token: str, line_no: int) -> object:
    t = token.strip()
    if t == "" or t == "~" or t.lower() == "null":
        return None
    if t.lower() == "true":
        return True
    if t.lower() == "false":
        return False
    if len(t) >= 2 and t[0] == "'" and t[-1] == "'":
        return t[1:-1].replace("''", "'")
    if len(t) >= 2 and t[0] == '"' and t[-1] == '"':
        try:
            return json.loads(t)
        except ValueError as exc:
            raise YamlishError(f"line {line_no}: bad double-quoted scalar {t!r}: {exc}") from exc
    if t[0] in "[{":
        try:
            return json.loads(t)
        except ValueError as exc:
            raise YamlishError(
                f"line {line_no}: flow collection must be written as JSON, got {t!r}: {exc}"
            ) from exc
    if t[0] in "&*!":
        raise YamlishError(
            f"line {line_no}: anchors, aliases and tags are outside the supported subset ({t!r})"
        )
    if _INT_RE.match(t):
        return int(t)
    if _FLOAT_RE.match(t) and ("." in t or "e" in t or "E" in t):
        return float(t)
    return t


def _split_key(text: str, line_no: int) -> tuple[str, str] | None:
    """Split 'key: rest' honouring quotes. Returns None when there is no key."""
    quote = None
    i = 0
    while i < len(text):
        ch = text[i]
        if quote is None:
            if ch in ("'", '"'):
                quote = ch
            elif ch in "[{":
                return None
            elif ch == ":":
                if i + 1 == len(text) or text[i + 1] == " ":
                    key_raw = text[:i].strip()
                    if not key_raw:
                        raise YamlishError(f"line {line_no}: empty mapping key")
                    key = _scalar(key_raw, line_no)
                    if not isinstance(key, str):
                        key = key_raw.strip("'\"")
                    return str(key), text[i + 1 :].strip()
        else:
            if ch == "\\" and quote == '"':
                i += 2
                continue
            if ch == quote:
                quote = None
        i += 1
    return None


class _Parser:
    def __init__(self, lines: list[_Line]) -> None:
        self.lines = lines
        self.pos = 0

    def peek(self) -> _Line | None:
        return self.lines[self.pos] if self.pos < len(self.lines) else None

    def parse_document(self) -> object:
        if not self.lines:
            return None
        return self.parse_block(self.lines[0].indent)

    @staticmethod
    def _is_sequence_entry(line: _Line) -> bool:
        return line.text == "-" or line.text.startswith("- ")

    def parse_block(self, indent: int) -> object:
        line = self.peek()
        if line is None:
            return None
        if self._is_sequence_entry(line):
            return self.parse_sequence(indent)
        return self.parse_mapping(indent)

    def parse_sequence(self, indent: int) -> list:
        out: list = []
        while True:
            line = self.peek()
            if line is None or line.indent < indent:
                break
            if line.indent > indent:
                raise YamlishError(
                    f"line {line.no}: unexpected indentation inside a sequence at column {indent}"
                )
            if not self._is_sequence_entry(line):
                break
            after_dash = line.text[1:]
            lead = len(after_dash) - len(after_dash.lstrip(" "))
            rest = after_dash.strip()
            # The column at which `rest` begins on the original line.
            item_indent = indent + 1 + lead
            self.pos += 1
            if rest == "":
                nxt = self.peek()
                if nxt is not None and nxt.indent > indent:
                    out.append(self.parse_block(nxt.indent))
                else:
                    out.append(None)
                continue
            split = _split_key(rest, line.no)
            if split is None:
                out.append(self._value_for(rest, line, item_indent))
            else:
                # Re-present the item body as an ordinary mapping line so a
                # multi-key inline item ("- a: 1" then "  b: 2") parses as one map.
                self.lines.insert(self.pos, _Line(line.no, item_indent, rest))
                out.append(self.parse_mapping(item_indent))
        return out

    def parse_mapping(self, indent: int) -> dict:
        out: dict = {}
        while True:
            line = self.peek()
            if line is None or line.indent < indent:
                break
            if line.indent > indent:
                raise YamlishError(
                    f"line {line.no}: unexpected indentation inside a mapping at column {indent}"
                )
            if self._is_sequence_entry(line):
                break
            split = _split_key(line.text, line.no)
            if split is None:
                raise YamlishError(f"line {line.no}: expected 'key: value', got {line.text!r}")
            key, rest = split
            if key in out:
                raise YamlishError(f"line {line.no}: duplicate mapping key {key!r}")
            self.pos += 1
            out[key] = self._value_for(rest, line, indent)
        return out

    def _value_for(self, rest: str, line: _Line, indent: int) -> object:
        if rest in ("|", "|-", "|+", ">", ">-", ">+"):
            return self._block_scalar(rest, indent)
        if rest != "":
            return _scalar(rest, line.no)
        nxt = self.peek()
        if nxt is None or nxt.indent <= indent:
            return None
        return self.parse_block(nxt.indent)

    def _block_scalar(self, marker: str, indent: int) -> str:
        folded = marker[0] == ">"
        chomp = marker[1] if len(marker) > 1 else ""
        collected: list[str] = []
        base: int | None = None
        while self.pos < len(self.lines):
            line = self.lines[self.pos]
            if line.indent <= indent:
                break
            if base is None:
                base = line.indent
            collected.append(" " * (line.indent - base) + line.text)
            self.pos += 1
        if not collected:
            return ""
        body = ("\n".join(collected)) if not folded else (" ".join(collected))
        if chomp == "-":
            return body
        return body + "\n"


def loads(text: str) -> object:
    """Parse a YAML-subset document. Returns None for an empty document."""
    if text.lstrip().startswith(("{", "[")):
        return json.loads(text)
    parser = _Parser(_tokenise(text))
    value = parser.parse_document()
    remaining = parser.peek()
    if remaining is not None:
        raise YamlishError(f"line {remaining.no}: trailing content {remaining.text!r}")
    return value


def load(path) -> object:
    with open(path, "r", encoding="utf-8") as fh:
        return loads(fh.read())


# --------------------------------------------------------------------------- dump

_PLAIN_SAFE = re.compile(r"^[A-Za-z_][A-Za-z0-9_.\-/]*$")
_RESERVED_WORDS = {
    "null",
    "Null",
    "NULL",
    "true",
    "True",
    "TRUE",
    "false",
    "False",
    "FALSE",
    "yes",
    "no",
    "on",
    "off",
    "y",
    "n",
    "~",
}


def _needs_quotes(s: str) -> bool:
    if s == "" or s in _RESERVED_WORDS:
        return True
    if s != s.strip():
        return True
    if s[0] in "-?:,[]{}#&*!|>'\"%@`":
        return True
    if ": " in s or " #" in s or "\n" in s:
        return True
    if _INT_RE.match(s) or _FLOAT_RE.match(s):
        return True
    return not _PLAIN_SAFE.match(s) and any(c in s for c in ":#{}[],&*!|>'\"%@`")


def _emit_scalar(value: object) -> str:
    if value is None:
        return "null"
    if value is True:
        return "true"
    if value is False:
        return "false"
    if isinstance(value, int):
        return str(value)
    if isinstance(value, float):
        return repr(value)
    if isinstance(value, str):
        if _needs_quotes(value):
            return json.dumps(value)
        return value
    raise YamlishError(f"cannot emit {type(value).__name__} as a YAML scalar")


def _emit(value: object, indent: int, out: list[str]) -> None:
    pad = " " * indent
    if isinstance(value, dict):
        if not value:
            raise YamlishError("internal: empty mapping must be emitted by its parent")
        for key, item in value.items():
            key_text = _emit_scalar(str(key))
            if isinstance(item, dict) and item:
                out.append(f"{pad}{key_text}:")
                _emit(item, indent + 2, out)
            elif isinstance(item, list) and item:
                out.append(f"{pad}{key_text}:")
                _emit(item, indent + 2, out)
            elif isinstance(item, dict):
                out.append(f"{pad}{key_text}: {{}}")
            elif isinstance(item, list):
                out.append(f"{pad}{key_text}: []")
            else:
                out.append(f"{pad}{key_text}: {_emit_scalar(item)}")
        return
    if isinstance(value, list):
        if not value:
            raise YamlishError("internal: empty sequence must be emitted by its parent")
        for item in value:
            if isinstance(item, dict) and item:
                sub: list[str] = []
                _emit(item, indent + 2, sub)
                first = sub[0].lstrip(" ")
                out.append(f"{pad}- {first}")
                out.extend(sub[1:])
            elif isinstance(item, list) and item:
                sub = []
                _emit(item, indent + 2, sub)
                first = sub[0].lstrip(" ")
                out.append(f"{pad}- {first}")
                out.extend(sub[1:])
            elif isinstance(item, dict):
                out.append(f"{pad}- {{}}")
            elif isinstance(item, list):
                out.append(f"{pad}- []")
            else:
                out.append(f"{pad}- {_emit_scalar(item)}")
        return
    out.append(f"{pad}{_emit_scalar(value)}")


def dumps(value: object) -> str:
    """Emit a YAML-subset document. Round-trips through `loads`."""
    if value is None:
        return "null\n"
    if isinstance(value, dict) and not value:
        return "{}\n"
    if isinstance(value, list) and not value:
        return "[]\n"
    out: list[str] = []
    _emit(value, 0, out)
    return "\n".join(out) + "\n"


def dump(value: object, path) -> None:
    with open(path, "w", encoding="utf-8") as fh:
        fh.write(dumps(value))
