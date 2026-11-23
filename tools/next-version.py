#!/usr/bin/env python3
"""Work out the next release version, and write it into the crates.

The release workflow runs this on every push to main:

    python tools/next-version.py --bump patch --apply

* No `v*` tag exists yet: the version in `crates/ffe-rs/Cargo.toml` is the first release, as is.
* The Cargo version is newer than the latest tag (someone bumped it by hand): it is the release.
* Otherwise the latest tag is bumped: `patch` by default, `minor` or `major` when asked
  (`#minor` or `#major` in the commit message, or the workflow's input).

`--apply` writes the version into `ffe-rs` and `ffe-launcher` (`Cargo.toml` and `Cargo.lock`),
which must stay equal: the launcher's updater compares its own version with the release tag.
Prints `version=X.Y.Z`, `tag=vX.Y.Z` and `changed=true|false` lines, ready for $GITHUB_OUTPUT.
"""
import argparse
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CRATES = ("ffe-rs", "ffe-launcher")


def parse(text):
    """'v0.9.1' or '0.9.1' as (0, 9, 1); None when it is not a plain release version."""
    match = re.fullmatch(r"v?(\d+)\.(\d+)\.(\d+)", text.strip())
    return tuple(int(part) for part in match.groups()) if match else None


def bump(version, kind):
    major, minor, patch = version
    if kind == "major":
        return (major + 1, 0, 0)
    if kind == "minor":
        return (major, minor + 1, 0)
    return (major, minor, patch + 1)


def next_version(latest_tag, cargo, kind):
    """The version to release, and whether the crates must change to it."""
    cargo_version = parse(cargo)
    if cargo_version is None:
        raise ValueError(f"unreadable Cargo version {cargo!r}")
    latest = parse(latest_tag) if latest_tag else None
    if latest is None or cargo_version > latest:
        return cargo_version, False
    chosen = bump(latest, kind)
    return chosen, chosen != cargo_version


def bump_kind(message, requested="auto"):
    if requested in ("major", "minor", "patch"):
        return requested
    lowered = message.lower()
    if "#major" in lowered:
        return "major"
    if "#minor" in lowered:
        return "minor"
    return "patch"


def cargo_version(root=ROOT):
    text = (root / "crates" / "ffe-rs" / "Cargo.toml").read_text(encoding="utf-8")
    match = re.search(r'(?m)^version\s*=\s*"([^"]+)"', text)
    if not match:
        raise ValueError("no version in crates/ffe-rs/Cargo.toml")
    return match.group(1)


def apply_version(version, root=ROOT):
    """Write X.Y.Z into both crates' manifests and the lock file."""
    new = ".".join(str(part) for part in version)
    for crate in CRATES:
        path = root / "crates" / crate / "Cargo.toml"
        text = path.read_text(encoding="utf-8")
        text, count = re.subn(r'(?m)^version\s*=\s*"[^"]+"', f'version = "{new}"', text, count=1)
        if count != 1:
            raise ValueError(f"no version line in {path}")
        path.write_text(text, encoding="utf-8", newline="")
    lock = root / "Cargo.lock"
    text = lock.read_text(encoding="utf-8")
    for crate in CRATES:
        pattern = rf'(\[\[package\]\]\nname = "{re.escape(crate)}"\nversion = ")[^"]+(")'
        text, count = re.subn(pattern, rf"\g<1>{new}\g<2>", text, count=1)
        if count != 1:
            raise ValueError(f"{crate} is not in Cargo.lock")
    lock.write_text(text, encoding="utf-8", newline="")


def latest_tag():
    out = subprocess.run(
        ["git", "tag", "--list", "v*", "--sort=-v:refname"],
        cwd=ROOT, capture_output=True, text=True, check=True,
    ).stdout.split()
    return next((tag for tag in out if parse(tag)), None)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--bump", default="auto", choices=["auto", "patch", "minor", "major"])
    parser.add_argument("--message", default="", help="the commit message (looked at for #minor/#major)")
    parser.add_argument("--apply", action="store_true", help="write the version into the crates")
    args = parser.parse_args(argv)
    version, changed = next_version(latest_tag(), cargo_version(), bump_kind(args.message, args.bump))
    if changed and args.apply:
        apply_version(version)
    text = ".".join(str(part) for part in version)
    print(f"version={text}")
    print(f"tag=v{text}")
    print(f"changed={'true' if changed else 'false'}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
