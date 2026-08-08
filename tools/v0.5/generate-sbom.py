#!/usr/bin/env python3
"""Generate deterministic CycloneDX 1.5 JSON and license inventory from Cargo metadata."""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path
from urllib.parse import quote


def cargo_metadata(repository_root: Path) -> dict[str, object]:
    completed = subprocess.run(
        ["cargo", "metadata", "--locked", "--format-version", "1"],
        cwd=repository_root,
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(completed.stdout)


def purl(name: str, version: str) -> str:
    return f"pkg:cargo/{quote(name, safe='')}@{quote(version, safe='.+-')}"


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repository-root", type=Path, default=Path.cwd())
    parser.add_argument("--sbom", type=Path, required=True)
    parser.add_argument("--licenses", type=Path, required=True)
    args = parser.parse_args()
    root = args.repository_root.resolve()
    metadata = cargo_metadata(root)

    packages = metadata["packages"]
    assert isinstance(packages, list)
    workspace_members = set(metadata["workspace_members"])
    by_id = {package["id"]: package for package in packages}

    components: list[dict[str, object]] = []
    license_rows: list[dict[str, object]] = []
    bom_ref_by_id: dict[str, str] = {}
    for package in sorted(packages, key=lambda item: (item["name"], item["version"], item["id"])):
        name = package["name"]
        version = package["version"]
        bom_ref = purl(name, version)
        if bom_ref in bom_ref_by_id.values():
            # Cargo cannot resolve two distinct packages with the exact same registry
            # name/version. Keep this fail closed for non-registry source collisions.
            source = package.get("source") or "workspace"
            bom_ref = f"{bom_ref}?source={quote(str(source), safe='')}"
        bom_ref_by_id[package["id"]] = bom_ref
        license_expression = package.get("license")
        component: dict[str, object] = {
            "type": "library" if package["id"] not in workspace_members else "application",
            "bom-ref": bom_ref,
            "name": name,
            "version": version,
            "purl": purl(name, version),
            "properties": [
                {
                    "name": "aligngauge:cargo-source",
                    "value": package.get("source") or "workspace",
                }
            ],
        }
        if license_expression:
            component["licenses"] = [{"expression": license_expression}]
        components.append(component)
        license_rows.append(
            {
                "name": name,
                "version": version,
                "workspace": package["id"] in workspace_members,
                "source": package.get("source") or "workspace",
                "license": license_expression,
            }
        )

    resolve = metadata.get("resolve") or {}
    nodes = resolve.get("nodes") or []
    dependencies: list[dict[str, object]] = []
    for node in sorted(nodes, key=lambda item: bom_ref_by_id[item["id"]]):
        dependencies.append(
            {
                "ref": bom_ref_by_id[node["id"]],
                "dependsOn": sorted(
                    bom_ref_by_id[dependency]
                    for dependency in node.get("dependencies", [])
                ),
            }
        )

    sbom = {
        "bomFormat": "CycloneDX",
        "specVersion": "1.5",
        "version": 1,
        "metadata": {
            "component": {
                "type": "application",
                "name": "aligngauge",
            },
            "tools": {
                "components": [
                    {
                        "type": "application",
                        "name": "tools/v0.5/generate-sbom.py",
                        "version": "1",
                    }
                ]
            },
        },
        "components": components,
        "dependencies": dependencies,
    }
    licenses = {
        "schema": "aligngauge-license-inventory-v1",
        "packages": license_rows,
    }

    for path, payload in [(args.sbom, sbom), (args.licenses, licenses)]:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(
            json.dumps(payload, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )


if __name__ == "__main__":
    main()
