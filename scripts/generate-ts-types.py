#!/usr/bin/env python3
"""Generate the published TypeScript tool types from the server's own schemas.

The MCP tool argument types are declared once, in Rust, with `schemars`.
Transcribing them into a `.d.ts` by hand puts a second declaration in the
repo that nothing keeps in step — and a stale *type* is worse than a stale
doc, because it compiles. This repo has already shipped that failure: the
plugin documentation described three tools for a six-tool server and named
three tools no server has ever registered (#161).

So: `dump-tool-schemas` emits the schemas the server actually registers,
and this script turns them into declarations. CI regenerates and fails on
any diff, which is what makes the types a consequence of the server rather
than a promise about it.

Stdlib only, deliberately. A generator that needs `npm install` to run is a
generator that stops being run.

Usage:
    cargo run -p rgaa-mcp --bin dump-tool-schemas > types/schemas.json
    python3 scripts/generate-ts-types.py
"""

from __future__ import annotations

import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
SCHEMAS = ROOT / "types" / "schemas.json"
OUT = ROOT / "types" / "src" / "tools.d.ts"

HEADER = """// GENERATED FILE — DO NOT EDIT.
//
// Produced by scripts/generate-ts-types.py from types/schemas.json, which is
// itself produced by:
//
//     cargo run -p rgaa-mcp --bin dump-tool-schemas > types/schemas.json
//
// Edit the Rust types and regenerate. CI fails if this file does not match
// what the server currently registers.

"""


def pascal(name: str) -> str:
    return "".join(part.capitalize() for part in re.split(r"[_\-\s]+", name) if part)


class Emitter:
    """Turns the subset of JSON Schema that `schemars` emits into TypeScript.

    Anything outside that subset becomes `unknown` rather than a guess. An
    over-eager mapping produces a type that type-checks and lies; `unknown`
    forces the caller to look, which is the honest failure.
    """

    def __init__(self, defs: dict):
        self.defs = defs
        self.emitted: dict[str, str] = {}

    def resolve(self, schema) -> dict:
        if not isinstance(schema, dict):
            return {}
        ref = schema.get("$ref")
        if not ref:
            return schema
        name = ref.rsplit("/", 1)[-1]
        return self.defs.get(name, {})

    def type_of(self, schema, hint: str = "") -> str:
        # JSON Schema allows a subschema to be a bare boolean: `true` accepts
        # any value, `false` accepts none. schemars emits `true` for a field
        # typed as an arbitrary JSON value (GuidedStepDto's `expected`).
        # Treating that as an object crashes; treating it as `unknown` is the
        # honest reading.
        if schema is True:
            return "unknown"
        if schema is False:
            return "never"
        if not isinstance(schema, dict) or not schema:
            return "unknown"

        # `const: X` is a single-value type. schemars uses it for the tag of
        # an internally-tagged enum, so without this every variant collapses
        # to `string` and the discriminated union stops discriminating.
        if "const" in schema:
            return json.dumps(schema["const"])

        if "$ref" in schema:
            name = schema["$ref"].rsplit("/", 1)[-1]
            if name in self.defs:
                self.emit_named(name, self.defs[name])
                return name
            return "unknown"

        for combinator in ("anyOf", "oneOf"):
            if combinator in schema:
                parts = [self.type_of(s, hint) for s in schema[combinator]]
                # schemars renders Option<T> as anyOf [T, null]; collapse the
                # duplicate rather than emitting `T | null | null`.
                seen: list[str] = []
                for p in parts:
                    if p not in seen:
                        seen.append(p)
                return " | ".join(seen) if seen else "unknown"

        if "allOf" in schema and len(schema["allOf"]) == 1:
            return self.type_of(schema["allOf"][0], hint)

        if "enum" in schema:
            return " | ".join(json.dumps(v) for v in schema["enum"])

        kind = schema.get("type")
        if isinstance(kind, list):
            # e.g. ["string", "null"]
            return " | ".join(
                self.type_of({**schema, "type": k}, hint) for k in kind
            )

        if kind == "string":
            return "string"
        if kind in ("integer", "number"):
            return "number"
        if kind == "boolean":
            return "boolean"
        if kind == "null":
            return "null"
        if kind == "array":
            return f"Array<{self.type_of(schema.get('items', {}), hint)}>"
        if kind == "object" or "properties" in schema:
            return self.object_literal(schema, hint)
        return "unknown"

    def object_literal(self, schema, hint: str, indent: str = "  ") -> str:
        if not isinstance(schema, dict):
            return self.type_of(schema, hint)
        props = schema.get("properties") or {}
        if not props:
            extra = schema.get("additionalProperties")
            if isinstance(extra, dict):
                return f"Record<string, {self.type_of(extra, hint)}>"
            return "Record<string, unknown>"
        required = set(schema.get("required") or [])
        lines = ["{"]
        for key, sub in props.items():
            doc = (sub.get("description") or "").strip() if isinstance(sub, dict) else ""
            if doc:
                for line in doc.splitlines():
                    lines.append(f"{indent}/** {line.strip()} */" if line.strip() else f"{indent}/** */")
            optional = "" if key in required else "?"
            lines.append(f"{indent}{json.dumps(key)}{optional}: {self.type_of(sub, hint)};")
        lines.append(indent[:-2] + "}")
        return "\n".join(lines)

    def emit_named(self, name: str, schema) -> None:
        if name in self.emitted:
            return
        self.emitted[name] = ""  # reserve first: schemars types can be cyclic
        inner = schema
        if isinstance(schema, dict) and "$ref" in schema:
            # Drop the key rather than blanking it: `"$ref" in schema` is
            # true for a None value too, so blanking it re-enters the $ref
            # branch and recurses forever.
            inner = {k: v for k, v in schema.items() if k != "$ref"}
        body = self.type_of(inner, name)
        doc = (schema.get("description") or "").strip()
        prefix = f"/** {doc} */\n" if doc else ""
        self.emitted[name] = f"{prefix}export type {name} = {body};\n"


def main() -> int:
    if not SCHEMAS.exists():
        print(
            f"missing {SCHEMAS.relative_to(ROOT)} — run:\n"
            "  cargo run -p rgaa-mcp --bin dump-tool-schemas > types/schemas.json",
            file=sys.stderr,
        )
        return 1

    document = json.loads(SCHEMAS.read_text())
    tools = document.get("tools") or []
    if not tools:
        print("schemas.json lists no tools; refusing to emit an empty file", file=sys.stderr)
        return 1

    chunks: list[str] = [HEADER]
    names: list[str] = []
    bodies: list[str] = []

    for tool in tools:
        name = tool["name"]
        names.append(name)
        schema = tool.get("inputSchema") or {}
        defs = schema.get("$defs") or schema.get("definitions") or {}
        emitter = Emitter(defs)
        args_type = f"{pascal(name)}Arguments"
        body = emitter.object_literal(schema, args_type)
        doc = (tool.get("description") or "").strip()

        block = ""
        for named in emitter.emitted.values():
            if named:
                block += named + "\n"
        if doc:
            block += "/**\n"
            for line in doc.splitlines():
                block += f" * {line}\n"
            block += " */\n"
        block += f"export type {args_type} = {body};\n"
        bodies.append(block)

    chunks.append(
        "/** Every tool name the server registers. */\n"
        "export type RgaaToolName =\n"
        + "\n".join(f"  | {json.dumps(n)}" for n in names)
        + ";\n\n"
    )
    chunks.append("\n".join(bodies))
    chunks.append(
        "\n/** Maps a tool name to its argument type. */\n"
        "export interface RgaaToolArguments {\n"
        + "".join(
            f"  {json.dumps(n)}: {pascal(n)}Arguments;\n" for n in names
        )
        + "}\n"
    )

    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text("".join(chunks))
    print(f"wrote {OUT.relative_to(ROOT)} ({len(names)} tools: {', '.join(names)})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
