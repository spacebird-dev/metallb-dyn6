#!/usr/bin/env python3
"""chart_version_from_cargo

This script syncs a helm charts version and AppVersion with an existing Cargo.toml file belong to a crate.
It ensures that "appVersion" matches the version defined in the crate.
The charts "version" field is updated to the next major/minor/patch release whenever the crates version changes.
"""

import argparse
import os
from pathlib import Path

from semver import VersionInfo
import toml
import yaml


def main():
    parser = argparse.ArgumentParser(formatter_class=argparse.ArgumentDefaultsHelpFormatter)
    parser.add_argument(
        "--chart-directory",
        help="Path to the chart directory", default=os.getcwd())
    parser.add_argument(
        "--cargo_toml", help="Path to the Cargo.toml file to read the app version from")
    parser.add_argument(
        "--dry-run", help="Don't modify the chart, only show what would happen", action="store_true")
    args = parser.parse_args()

    cargo_app_ver = None
    chart_data = None
    chart_directory = Path(args.chart_directory)

    with open(Path(args.cargo_toml), encoding="utf-8") as f:
        data = toml.load(f)
        try:
            # Crate
            cargo_app_ver = VersionInfo.parse(data["package"]["version"])
        except KeyError:
            # Workspace
            cargo_app_ver = VersionInfo.parse(data["workspace"]["package"]["version"])
        print(f"Read cargo package version: {cargo_app_ver}")

    with open(Path(chart_directory).joinpath("Chart.yaml"), encoding="utf-8") as f:
        chart_data = yaml.safe_load(f)

    current_chart_ver = VersionInfo.parse(str(chart_data["version"]))
    current_chart_app = VersionInfo.parse(str(chart_data["appVersion"]))
    print(f"Read chart version: {current_chart_ver}")
    print(f"Read chart appVersion: {current_chart_app}")

    if current_chart_app == cargo_app_ver:
        print("appVersion is up to date - nothing to do")
        return
    elif cargo_app_ver.prerelease:
        print("Cargo.toml contains prerelease version, not updating chart")
        return
    elif current_chart_app > cargo_app_ver:
        raise ValueError("appVersion is larger than version in Cargo.toml, please resolve manually")

    if current_chart_app.major < cargo_app_ver.major:
        chart_data["version"] = str(current_chart_ver.next_version("major"))
    elif current_chart_app.minor < cargo_app_ver.minor:
        chart_data["version"] = str(current_chart_ver.next_version("minor"))
    elif current_chart_app.patch < cargo_app_ver.patch:
        chart_data["version"] = str(current_chart_ver.next_version("patch"))
    else:
        print(f"Could not determine version mismatch between Cargo.toml ({cargo_app_ver}) "
              f"and Chart.yaml ({current_chart_ver})")
        return
    chart_data["appVersion"] = str(cargo_app_ver)

    print(f"New chart version: {chart_data['version']}")
    print(f"New chart appVersion: {chart_data['appVersion']}")

    if not args.dry_run:
        with open((chart_directory).joinpath("Chart.yaml"), "w", encoding="utf-8") as f:
            yaml.dump(chart_data, f, allow_unicode=True)
        print("Saved Chart.yaml")


if __name__ == "__main__":
    main()
