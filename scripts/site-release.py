"""Render verified GitHub release downloads into the static website."""

import argparse
import json
import re
from datetime import datetime, timezone
from pathlib import Path
from textwrap import indent

DOWNLOAD_ROOT = "https://github.com/mpakus/1537paperstreet/releases/download"
PACKAGES = (
    ("macos", "macOS", "universal.dmg", "DMG"),
    ("linux", "Linux", "amd64.deb", "DEB"),
    ("windows", "Windows", "x64-setup.exe", "EXE"),
)


def render_release(page, release):
    """Return updated HTML only for a complete, stable, published release."""
    tag = release["tag_name"]
    if release["draft"] or release["prerelease"] or not re.fullmatch(r"v\d+\.\d+\.\d+", tag):
        raise ValueError("Downloads require a published stable vX.Y.Z release")
    version = tag[1:]
    published = datetime.fromisoformat(release["published_at"].replace("Z", "+00:00"))
    if published.tzinfo is None:
        raise ValueError("The release publication date must include a timezone")
    published = published.astimezone(timezone.utc)
    date = f"{published:%B} {published.day}, {published.year}"
    sections = {
        "info": (
            '<p class="release-version">\n'
            f'  Version {version} · Released <time datetime="{published:%Y-%m-%d}">{date}</time>\n'
            '</p>'
        )
    }
    for key, label, suffix, file_format in PACKAGES:
        name = f"1537paperstreet_{version}_{suffix}"
        matches = [asset for asset in release["assets"] if asset["name"] == name]
        if len(matches) != 1:
            raise ValueError(f"Expected exactly one installer: {name}")
        asset = matches[0]
        url = f"{DOWNLOAD_ROOT}/{tag}/{name}"
        if asset["state"] != "uploaded" or asset["size"] <= 0:
            raise ValueError(f"Installer is not fully uploaded: {name}")
        if asset["browser_download_url"] != url:
            raise ValueError(f"Unexpected download URL for {name}")
        sections[key] = (
            '<a\n'
            '  class="button"\n'
            f'  href="{url}"\n'
            '  download\n'
            f'  >Download for {label}</a\n'
            '>\n'
            f'<p class="fine">{file_format} · {asset["size"] / 1_000_000:.1f} MB</p>'
        )
    for key, content in sections.items():
        pattern = rf"(?ms)^([ \t]*)<!-- release:{key} -->.*?<!-- /release:{key} -->"

        def replace(match):
            prefix = match[1]
            return (
                f"{prefix}<!-- release:{key} -->\n"
                f"{indent(content, prefix)}\n"
                f"{prefix}<!-- /release:{key} -->"
            )

        page, count = re.subn(pattern, replace, page)
        if count != 1:
            raise ValueError(f"Expected exactly one release:{key} marker in the page")
    return page


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("release", type=Path, help="GitHub release API JSON")
    parser.add_argument("page", type=Path, help="Website index.html to update")
    args = parser.parse_args()
    try:
        rendered = render_release(
            args.page.read_text(encoding="utf-8"),
            json.loads(args.release.read_text(encoding="utf-8")),
        )
    except (KeyError, TypeError, ValueError) as error:
        parser.exit(1, f"Cannot publish download links: {error}\n")
    args.page.write_text(rendered, encoding="utf-8")
