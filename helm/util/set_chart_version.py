#!/usr/bin/env python
"""This script updates the Chart version in a Chart.yaml file to the supplied value
"""

import argparse
from pathlib import Path

from semver import VersionInfo
import yaml


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "CHART_DIRECTORY", help="Path to the chart directory containing Chart.yaml", default="./")
    parser.add_argument(
        "FIELD", choices=["app", "chart"], help="Which version field to update")
    parser.add_argument("VERSION", help="Version number to set")
    args = parser.parse_args()

    chart_path = Path(args.CHART_DIRECTORY)
    version = VersionInfo.parse(args.VERSION)

    update_chart_field = args.FIELD == "chart"
    update_app_field = args.FIELD == "app"

    contents = None
    with open(chart_path / "Chart.yaml", encoding="utf-8") as f:
        contents = yaml.safe_load(f)

    if update_chart_field:
        print(f"Updating chart version: {contents['version']} -> {version}")
        contents["version"] = str(version)
    elif update_app_field:
        print(f"Updating app version: {contents['appVersion']} -> {version}")
        contents["appVersion"] = str(version)
    with open(chart_path / "Chart.yaml", "w", encoding="utf-8") as f:
        yaml.dump(contents, f)


if __name__ == "__main__":
    main()
