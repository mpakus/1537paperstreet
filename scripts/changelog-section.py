#!/usr/bin/env python3
"""Print the Keep a Changelog section for a version (for GitHub Releases)."""

from __future__ import annotations

import argparse
import io
import re
import sys
from pathlib import Path

SAMPLE = """# Changelog

## [Unreleased]

### Added

- not this

## [0.1.0] - 2026-08-19

### Added

- first release

## [0.0.1] - 2026-01-01

- older

[Unreleased]: https://example.com/compare/v0.1.0...HEAD
[0.1.0]: https://example.com/releases/tag/v0.1.0
"""


def normalize_version(raw: str) -> str:
    text = raw.strip()
    if text.startswith("v") or text.startswith("V"):
        return text[1:]
    return text


def section_for(changelog: str, version: str) -> str:
    version = normalize_version(version)
    heading = re.compile(
        rf"^## \[{re.escape(version)}\](?:\s+-.*)?\s*$",
        re.MULTILINE,
    )
    match = heading.search(changelog)
    if match is None:
        raise SystemExit(f"CHANGELOG.md has no section for [{version}]")

    start = match.end()
    next_heading = re.search(r"^## \[", changelog[start:], re.MULTILINE)
    next_link = re.search(r"^\[", changelog[start:], re.MULTILINE)
    end = len(changelog)
    if next_heading is not None:
        end = min(end, start + next_heading.start())
    if next_link is not None:
        end = min(end, start + next_link.start())

    title = match.group(0).strip()
    body = changelog[start:end].strip()
    if not body:
        raise SystemExit(f"CHANGELOG.md section [{version}] is empty")
    return f"{title}\n\n{body}\n"


SIGNED_FOOTER = """
---

Universal macOS build (Apple Silicon `arm64` + Intel `x86_64`), minimum macOS 12.0.

Signed with Developer ID and notarized by Apple. The notarization ticket is stapled to both the app and DMG for offline Gatekeeper verification.

Linux `.deb` (x86_64) and the Windows NSIS installer are on the same release. Those packages are not Apple-signed. In-app install still replaces the macOS `.app`; Linux and Windows open this page.
"""


def write_stdout(text: str) -> None:
    """Write release notes as UTF-8 bytes.

    GitHub's Windows runner opens Python stdout as cp1252. Text such as
    "Settings → …" cannot be encoded in that code page, and
    ``reconfigure`` does not stick when bash redirects stdout to a file.
    """
    encoded = text.encode("utf-8")
    buffer = getattr(sys.stdout, "buffer", None)
    if buffer is None:
        sys.stdout.write(text)
        return
    buffer.write(encoded)
    buffer.flush()


def self_test() -> None:
    got = section_for(SAMPLE, "v0.1.0")
    assert "## [0.1.0] - 2026-08-19" in got
    assert "first release" in got
    assert "not this" not in got
    assert "older" not in got
    try:
        section_for(SAMPLE, "9.9.9")
    except SystemExit:
        pass
    else:
        raise AssertionError("missing version should fail")
    notes = section_for(SAMPLE, "0.1.0") + SIGNED_FOOTER
    assert "notarized by Apple" in notes
    raw = io.BytesIO()
    wrapped = io.TextIOWrapper(raw, encoding="cp1252")
    previous = sys.stdout
    sys.stdout = wrapped
    try:
        write_stdout("Settings → Show table of contents\n")
    finally:
        sys.stdout = previous
        wrapped.detach()
    assert raw.getvalue().decode("utf-8") == "Settings → Show table of contents\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "version",
        nargs="?",
        help="Version or git tag, e.g. 0.1.0 or v0.1.0",
    )
    parser.add_argument(
        "--changelog",
        type=Path,
        default=Path("CHANGELOG.md"),
        help="Path to CHANGELOG.md",
    )
    parser.add_argument(
        "--self-test",
        action="store_true",
        help="Run built-in assertions and exit",
    )
    parser.add_argument(
        "--signed-footer",
        action="store_true",
        help="Append signing details used on GitHub Releases",
    )
    args = parser.parse_args(argv)

    if args.self_test:
        self_test()
        return 0

    if not args.version:
        parser.error("version is required unless --self-test")

    text = args.changelog.read_text(encoding="utf-8")
    notes = section_for(text, args.version)
    if args.signed_footer:
        notes += SIGNED_FOOTER
    write_stdout(notes)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
