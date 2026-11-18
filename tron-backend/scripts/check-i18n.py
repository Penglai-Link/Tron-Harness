#!/usr/bin/env python3
"""Validate Tron's centralized rust-i18n resources.

Checks:
  1. every `locales/*.yml` file is a strict `_version: 2` resource with exactly
     the supported locale tags and no duplicate/empty entries;
  2. every statically referenced translation key exists in the resources,
     including `__i18n__<key>` input-schema example sentinels;
  3. `tron_i18n::t!` is only used through the shared API with an explicit
     `locale = ...` argument;
  4. business crates never reach into `rust_i18n` directly and register the
     resource directory only once, in `crates/i18n`.

Run with `make be-i18n-check`. CI runs the same target. Only the Python
standard library is used so the check works on macOS, Linux and Windows.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

SUPPORTED_LOCALES = ("zh-CN", "en")
LOCALE_FILE_SUFFIXES = (".yml", ".yaml")
VERSION_LINE = re.compile(r"^_version:\s*(\d+)\s*$")
KEY_LINE = re.compile(r"^([A-Za-z0-9_][A-Za-z0-9_.\-]*):\s*$")
ENTRY_LINE = re.compile(r"^\s{2}([A-Za-z0-9_\-]+):\s*\"(.*)\"\s*$")
TRANSLATION_KEY = re.compile(r'^[a-z][a-z0-9_]*(?:\.[A-Za-z0-9_\-]+)+$')


class Error:
    def __init__(self) -> None:
        self.items: list[str] = []

    def add(self, message: str) -> None:
        self.items.append(message)

    def report(self, title: str) -> bool:
        if not self.items:
            return True
        print(f"{title}: {len(self.items)} problem(s)", file=sys.stderr)
        for item in self.items:
            print(f"  - {item}", file=sys.stderr)
        return False


def unescape(value: str) -> str:
    out: list[str] = []
    escaped = False
    for ch in value:
        if escaped:
            out.append({"n": "\n", "t": "\t", '"': '"', "\\": "\\"}.get(ch, ch))
            escaped = False
        elif ch == "\\":
            escaped = True
        else:
            out.append(ch)
    if escaped:
        out.append("\\")
    return "".join(out)


def parse_locale_file(path: Path, errors: Error) -> dict[str, dict[str, str]]:
    """Parse one strict `_version: 2` resource file."""
    messages: dict[str, dict[str, str]] = {}
    key: str | None = None
    seen_version = False
    for number, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = raw.rstrip()
        where = f"{path}:{number}"
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        if VERSION_LINE.match(line):
            if seen_version:
                errors.add(f"{where}: duplicate `_version`")
            seen_version = True
            if VERSION_LINE.match(line).group(1) != "2":
                errors.add(f"{where}: locale resources must use `_version: 2`")
            continue
        if match := ENTRY_LINE.match(line):
            if key is None:
                errors.add(f"{where}: locale entry without a key")
                continue
            locale, value = match.group(1), unescape(match.group(2))
            if locale not in SUPPORTED_LOCALES:
                errors.add(
                    f"{where}: unsupported locale `{locale}` "
                    f"(expected one of {', '.join(SUPPORTED_LOCALES)})"
                )
                continue
            if not value.strip():
                errors.add(f"{where}: empty translation for `{key}`/{locale}")
                continue
            messages.setdefault(key, {})[locale] = value
            continue
        if match := KEY_LINE.match(line):
            key = match.group(1)
            if key in messages:
                errors.add(f"{where}: duplicate key `{key}` in this file")
            continue
        errors.add(f"{where}: unsupported locale resource syntax: {line.strip()!r}")
    if not seen_version:
        errors.add(f"{path}: missing `_version: 2` header")
    for key, entries in messages.items():
        missing = [tag for tag in SUPPORTED_LOCALES if tag not in entries]
        if missing:
            errors.add(f"{path}: `{key}` is missing locale(s): {', '.join(missing)}")
    return messages


def collect_resources(root: Path, errors: Error) -> dict[str, Path]:
    locales = root / "locales"
    if not locales.is_dir():
        errors.add(f"{locales} does not exist; all translations belong there")
        return {}
    resources: dict[str, Path] = {}
    files = sorted(
        path
        for path in locales.rglob("*")
        if path.is_file() and path.suffix in LOCALE_FILE_SUFFIXES
    )
    if not files:
        errors.add(f"{locales} contains no locale resources")
    for path in files:
        for key in parse_locale_file(path, errors):
            if key in resources:
                errors.add(
                    f"{path}: key `{key}` is also defined in {resources[key]}"
                )
                continue
            resources[key] = path
    return resources


def strip_comments(source: str) -> str:
    """Remove Rust line/block comments while preserving strings and newlines."""
    out: list[str] = []
    index = 0
    size = len(source)
    in_string = False
    in_raw = 0
    while index < size:
        char = source[index]
        if in_string:
            out.append(char)
            index += 1
            if char == "\\" and index < size:
                out.append(source[index])
                index += 1
            elif char == '"':
                in_string = False
            continue
        if in_raw:
            if source.startswith('"' + "#" * in_raw, index):
                out.append('"' + "#" * in_raw)
                index += 1 + in_raw
                in_raw = 0
            else:
                out.append(char)
                index += 1
            continue
        if source.startswith("//", index):
            end = source.find("\n", index)
            if end == -1:
                break
            out.append("\n")
            index = end
            continue
        if source.startswith("/*", index):
            end = source.find("*/", index + 2)
            if end == -1:
                break
            out.append("\n" * source.count("\n", index, end + 2))
            index = end + 2
            continue
        if char == '"':
            in_string = True
            out.append(char)
            index += 1
            continue
        if char == "r":
            match = re.match(r'r(#+)?"', source[index:])
            if match:
                hashes = match.group(1)
                in_raw = len(hashes) if hashes else 0
                out.append(source[index : index + match.end()])
                index += match.end()
                if in_raw == 0:
                    in_raw = 0
                    # r"..." is a normal raw string; treat as string until closing quote
                    end = source.find('"', index)
                    if end == -1:
                        break
                    out.append(source[index : end + 1])
                    index = end + 1
                continue
        out.append(char)
        index += 1
    return "".join(out)


def balanced_call(source: str, start: int) -> str | None:
    """Return the argument text of a macro call starting at `start`."""
    depth = 0
    index = start
    in_string = False
    while index < len(source):
        char = source[index]
        if in_string:
            if char == "\\":
                index += 2
                continue
            if char == '"':
                in_string = False
            index += 1
            continue
        if char == '"':
            in_string = True
        elif char in "([{":
            depth += 1
        elif char in ")]}":
            depth -= 1
            if depth == 0:
                return source[start + 1 : index]
        index += 1
    return None


def translation_keys_from_macro(root: Path, errors: Error) -> set[str]:
    used: set[str] = set()
    for path in rust_sources(root):
        relative = path.relative_to(root)
        source = strip_comments(path.read_text(encoding="utf-8", errors="replace"))
        if "rust_i18n::" in source and relative.parts[:2] != ("crates", "i18n"):
            for number, line in enumerate(source.splitlines(), 1):
                if "rust_i18n::" in line and "//" not in line:
                    errors.add(
                        f"{path}:{number}: use the shared `tron_i18n` API, not `rust_i18n` directly"
                    )
        for match in re.finditer(r"(?<![A-Za-z0-9_])t!\s*\(", source):
            arguments = balanced_call(source, match.end() - 1)
            if arguments is None:
                errors.add(f"{path}: unbalanced `t!` invocation")
                continue
            line = source.count("\n", 0, match.start()) + 1
            if not re.search(r"\blocale\s*=", arguments):
                errors.add(
                    f"{path}:{line}: `t!` must pass an explicit `locale = ...` argument"
                )
            first = re.match(r'\s*"((?:[^"\\]|\\.)*)"', arguments)
            if first:
                key = unescape(first.group(1))
                if not TRANSLATION_KEY.match(key):
                    errors.add(f"{path}:{line}: `{key}` is not a semantic translation key")
                used.add(key)
    return used


def rust_sources(root: Path):
    for base in (root / "src", root / "crates"):
        if not base.is_dir():
            continue
        for path in sorted(base.rglob("*.rs")):
            if "target" in path.parts:
                continue
            yield path


def declared_catalog_keys(root: Path, errors: Error) -> set[str]:
    """Collect keys referenced through `*_i18n_key: "..."` definitions and
    `__i18n__<key>` schema example sentinels."""
    keys: set[str] = set()
    i18n_key = re.compile(r'[A-Za-z_]*i18n_key:\s*"((?:[^"\\]|\\.)*)"')
    schema_key = re.compile(r'schema\(\s*"((?:[^"\\]|\\.)*)"')
    description_key = re.compile(
        r'x-i18n-description-key"\s*:\s*"((?:[^"\\]|\\.)*)"'
    )
    sentinel = re.compile(r'__i18n__((?:[A-Za-z0-9_]|\.)+)')
    registrations = 0
    for path in rust_sources(root):
        relative = path.relative_to(root)
        source = strip_comments(path.read_text(encoding="utf-8", errors="replace"))
        for pattern in (i18n_key, schema_key, description_key):
            for match in pattern.finditer(source):
                key = unescape(match.group(1))
                if TRANSLATION_KEY.match(key):
                    keys.add(key)
        for match in sentinel.finditer(source):
            keys.add(unescape(match.group(1)))
        if "i18n!(" in source and "rust_i18n::i18n!" in source:
            registrations += 1
            if relative.parts[:2] != ("crates", "i18n"):
                errors.add(
                    f"{path}: `i18n!` may only be registered once in crates/i18n"
                )
    if registrations != 1:
        errors.add(
            f"expected exactly one rust_i18n registration, found {registrations}"
        )
    for manifest in [root / "Cargo.toml", *sorted((root / "crates").glob("*/Cargo.toml"))]:
        if not manifest.is_file():
            continue
        text = manifest.read_text(encoding="utf-8")
        if "rust-i18n" in text and manifest.parent != root / "crates" / "i18n":
            errors.add(f"{manifest}: only crates/i18n may depend on rust-i18n directly")
    return keys


def main() -> int:
    root = Path(__file__).resolve().parent.parent
    errors = Error()
    resources = collect_resources(root, errors)
    used = translation_keys_from_macro(root, errors)
    used |= declared_catalog_keys(root, errors)
    for key in sorted(used):
        if key not in resources:
            errors.add(f"translation key `{key}` is used but not defined in locales/")
    if not errors.report("i18n check failed"):
        return 1
    print(
        f"i18n check passed: {len(resources)} keys across "
        f"{len(SUPPORTED_LOCALES)} locales ({', '.join(SUPPORTED_LOCALES)}), "
        f"{len(used)} keys referenced from Rust sources"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())