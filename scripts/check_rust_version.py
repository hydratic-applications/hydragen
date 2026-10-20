#!/usr/bin/env python3
"""Check Cargo's MSRV and local README claims without accessing the network."""

import argparse
from html.parser import HTMLParser
from pathlib import Path
import re
import sys
import tomllib
from urllib.parse import parse_qs, urlsplit


ROOT = Path(__file__).resolve().parents[1]
VERSION = re.compile(r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)")
BADGE_SOURCE = "https://raw.githubusercontent.com/hydratic-applications/hydragen/main/Cargo.toml"
BADGE_QUERY = "$.package['rust-version']"
BADGE_ALT = "Minimum supported Rust version on main"


class CheckError(ValueError):
    """A declaration or README claim needs a maintainer correction."""


def read_version(path):
    metadata = tomllib.loads(path.read_text(encoding="utf-8"))
    package = metadata.get("package")
    value = package.get("rust-version") if isinstance(package, dict) else None
    if not isinstance(value, str) or VERSION.fullmatch(value) is None:
        raise CheckError(
            "Cargo.toml: set [package].rust-version to a full stable ASCII version "
            "such as 1.89.0 (major.minor.patch), without aliases, leading zeroes, "
            "suffixes, or whitespace."
        )
    return value


class LinkedImages(HTMLParser):
    def __init__(self):
        super().__init__()
        self.links = []
        self.images = []

    def handle_starttag(self, tag, attrs):
        values = dict(attrs)
        if tag == "a":
            self.links.append(values.get("href"))
        elif tag == "img":
            self.images.append((attrs, self.links[-1] if self.links else None))

    def handle_startendtag(self, tag, attrs):
        self.handle_starttag(tag, attrs)
        self.handle_endtag(tag)

    def handle_endtag(self, tag):
        if tag == "a" and self.links:
            self.links.pop()


def check_readme(text, version):
    sections = re.findall(
        r"^## Build and run locally\s*\n(.*?)(?=^## |\Z)",
        text,
        flags=re.MULTILINE | re.DOTALL,
    )
    claims = re.findall(
        r"minimum supported Rust version\s+\(MSRV\)\s+is\s+\*\*([^*]+)\*\*",
        sections[0] if len(sections) == 1 else "",
    )
    if claims != [version]:
        raise CheckError(
            "README.md: update the Build and run locally section to say "
            f"'The minimum supported Rust version (MSRV) is **{version}**' "
            "exactly once, matching Cargo.toml."
        )

    parser = LinkedImages()
    parser.feed(text)
    candidates = []
    for attrs, link in parser.images:
        values = dict(attrs)
        alt = (values.get("alt") or "").casefold()
        src = values.get("src") or ""
        if "minimum supported rust version" in alt or "rust-version" in src or "label=MSRV" in src:
            candidates.append((attrs, link))
    if len(candidates) != 1:
        raise CheckError("README.md: include exactly one linked dynamic MSRV shield.")

    attrs, link = candidates[0]
    image = dict(attrs)
    if len(attrs) != len(image):
        raise CheckError("README.md: remove duplicate MSRV shield image attributes.")
    if image.get("alt") != BADGE_ALT:
        raise CheckError(f"README.md: set the MSRV shield alternative text to '{BADGE_ALT}'.")
    if link != "#build-and-run-locally" or image.get("height") != "22":
        raise CheckError("README.md: link the 22px MSRV shield to #build-and-run-locally.")

    try:
        url = urlsplit(image.get("src") or "")
        query = parse_qs(url.query, keep_blank_values=True, strict_parsing=True)
    except ValueError as exc:
        raise CheckError("README.md: repair the MSRV shield's dynamic TOML URL.") from exc
    if (url.scheme, url.netloc, url.path, url.fragment) != (
        "https", "img.shields.io", "/badge/dynamic/toml", ""
    ):
        raise CheckError("README.md: use the HTTPS Shields.io /badge/dynamic/toml endpoint for MSRV.")
    expected = {
        "url": [BADGE_SOURCE],
        "query": [BADGE_QUERY],
        "label": ["Minimum Rust"],
        "style": ["for-the-badge"],
        "color": ["D34516"],
        "logo": ["rust"],
        "logoColor": ["white"],
    }
    if query != expected:
        raise CheckError(
            "README.md: configure the MSRV shield to read "
            f"{BADGE_SOURCE} with query {BADGE_QUERY}, label 'Minimum Rust', "
            "a white Rust logo, for-the-badge style, and color D34516; "
            "remove extra or duplicate parameters."
        )


def check(root):
    version = read_version(root / "Cargo.toml")
    check_readme((root / "README.md").read_text(encoding="utf-8"), version)
    return version


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT, help="checkout to check (default: script's repository)")
    parser.add_argument("--print-version", action="store_true", help="print only the validated version after all checks pass")
    args = parser.parse_args(argv)
    try:
        version = check(args.root)
    except (OSError, ValueError) as exc:
        print(f"Rust version check failed: {exc}", file=sys.stderr)
        return 1
    print(version if args.print_version else f"Rust version checks passed (MSRV {version}).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
