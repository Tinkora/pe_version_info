#!/usr/bin/env python3
"""Validate a release candidate version against the workspace manifest."""

import argparse
import re
import tomllib
from pathlib import Path


SEMVER = re.compile(
    r"^v(0|[1-9][0-9]*)\."
    r"(0|[1-9][0-9]*)\."
    r"(0|[1-9][0-9]*)"
    r"(?:-((?:0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*)"
    r"(?:\.(?:0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*))*))?"
    r"(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$"
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("version")
    parser.add_argument("--manifest", type=Path, default=Path("Cargo.toml"))
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    if SEMVER.fullmatch(args.version) is None:
        raise SystemExit("version must be a v-prefixed SemVer 2.0.0 value")

    with args.manifest.open("rb") as source:
        manifest = tomllib.load(source)
    workspace_version = manifest["workspace"]["package"]["version"]
    expected = f"v{workspace_version}"
    if args.version != expected:
        raise SystemExit(
            f"version {args.version} does not match workspace version {expected}"
        )
    print(f"validated {args.version}")


if __name__ == "__main__":
    main()
