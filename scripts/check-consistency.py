#!/usr/bin/env python3
"""Lightweight static consistency checks for the ViBao semantic locale layer.

This is deliberately not a Rust compiler replacement. It catches omissions in
small, repetitive registry/locale tables while the project is being developed
in environments without cargo/rustc.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
AST = ROOT / "vibao-ast" / "src" / "semantic"
LOCALE = ROOT / "vibaoc" / "src" / "locale"

DOMAINS = {
    "PropKey": (AST / "prop.rs", [LOCALE / "prop_vi.rs", LOCALE / "prop_en.rs"]),
    "ActionName": (AST / "action.rs", [LOCALE / "action_vi.rs", LOCALE / "action_en.rs"]),
    "FunctionName": (AST / "function.rs", [LOCALE / "function_vi.rs", LOCALE / "function_en.rs"]),
}


def read(path: Path) -> str:
    if not path.exists():
        raise RuntimeError(f"missing file: {path}")
    return path.read_text(encoding="utf-8")


def enum_variants(text: str, enum_name: str) -> set[str]:
    m = re.search(rf"pub enum {re.escape(enum_name)}\s*\{{(.*?)\n\}}", text, re.S)
    if not m:
        raise RuntimeError(f"cannot find enum {enum_name}")
    body = m.group(1)
    # Semantic enums here are unit variants. Ignore comments/doc comments.
    body = re.sub(r"//.*", "", body)
    body = re.sub(r"/\*.*?\*/", "", body, flags=re.S)
    return set(re.findall(r"(?m)^\s*([A-Z][A-Za-z0-9_]*)\s*,", body))


def mapped_variants(text: str, enum_name: str) -> set[str]:
    return set(re.findall(rf"=>\s*{re.escape(enum_name)}::([A-Z][A-Za-z0-9_]*)\b", text))


def method_variants(text: str, enum_name: str, method_name: str) -> set[str]:
    m = re.search(
        rf"impl\s+{re.escape(enum_name)}\s*\{{.*?pub const fn {re.escape(method_name)}.*?\{{(.*?)\n\s*}}\n}}",
        text,
        re.S,
    )
    if not m:
        raise RuntimeError(f"cannot find {enum_name}::{method_name}")
    return set(re.findall(r"Self::([A-Z][A-Za-z0-9_]*)\s*=>", m.group(1)))


def check_domain(name: str, enum_file: Path, locale_files: list[Path]) -> list[str]:
    variants = enum_variants(read(enum_file), name)
    problems: list[str] = []
    union: set[str] = set()
    for locale_file in locale_files:
        mapped = mapped_variants(read(locale_file), name)
        missing = variants - mapped
        extra = mapped - variants
        union |= mapped
        if missing:
            problems.append(f"{name}: {locale_file.name} missing: {', '.join(sorted(missing))}")
        if extra:
            problems.append(f"{name}: {locale_file.name} has unknown variants: {', '.join(sorted(extra))}")
    # Both locale tables should cover the semantic domain. Aliases are allowed
    # at the surface-name level, so this checks identity coverage only.
    missing_union = variants - union
    if missing_union:
        problems.append(f"{name}: no locale maps: {', '.join(sorted(missing_union))}")

    # Action/Function identities must also have a canonical runtime spelling.
    # This is deliberately static: it checks the repetitive mapping without
    # pretending to replace cargo/rustc.
    if name in {"ActionName", "FunctionName"}:
        method = method_variants(read(enum_file), name, "runtime_name")
        missing_runtime = variants - method
        extra_runtime = method - variants
        if missing_runtime:
            problems.append(f"{name}::runtime_name missing: {', '.join(sorted(missing_runtime))}")
        if extra_runtime:
            problems.append(f"{name}::runtime_name has unknown variants: {', '.join(sorted(extra_runtime))}")
    return problems


def main() -> int:
    problems: list[str] = []
    for name, (enum_file, locale_files) in DOMAINS.items():
        problems.extend(check_domain(name, enum_file, locale_files))

    mod = read(LOCALE / "mod.rs")
    for module in ("prop_vi", "prop_en", "action_vi", "action_en", "function_vi", "function_en"):
        if not re.search(rf"pub mod {module}\s*;", mod):
            problems.append(f"locale/mod.rs does not declare pub mod {module}")

    tables = read(ROOT / "vibaoc" / "src" / "lexer" / "tables.rs")
    for const_name in ("ALL_ACTION_SURFACE_NAMES_EN", "ALL_FUNCTION_SURFACE_NAMES_EN"):
        if not re.search(rf"const {const_name}\s*:", tables):
            problems.append(f"lexer/tables.rs missing {const_name}")
    if "ALL_ACTION_SURFACE_NAMES_EN.iter()" not in tables or "ALL_FUNCTION_SURFACE_NAMES_EN.iter()" not in tables:
        problems.append("lexer/tables.rs does not wire both English action/function surface tables into component_set()")

    if problems:
        print("ViBao consistency check: FAIL")
        for problem in problems:
            print(f"- {problem}")
        return 1

    print("ViBao consistency check: PASS")
    for name, (enum_file, _) in DOMAINS.items():
        print(f"- {name}: semantic variants covered by VI + EN locale tables")
    print("- locale/mod.rs: VI + EN modules wired")
    return 0


if __name__ == "__main__":
    sys.exit(main())
