#!/usr/bin/env python3
"""Dry-run packaging and validate release-plz's workspace publish order."""

from __future__ import annotations

import argparse
import json
import subprocess
import tarfile
import tomllib
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable


DEPENDENCY_TABLES = ("dependencies", "build-dependencies", "dev-dependencies")


@dataclass(frozen=True)
class Package:
    name: str
    version: str
    manifest_path: Path
    dependencies: tuple[dict[str, Any], ...]
    features: dict[str, list[str]]

    @property
    def directory(self) -> Path:
        return self.manifest_path.parent


def cargo_metadata() -> dict[str, Any]:
    output = subprocess.check_output(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        text=True,
    )
    return json.loads(output)


def publishable_packages(metadata: dict[str, Any]) -> list[Package]:
    workspace_members = set(metadata["workspace_members"])
    packages = []
    for raw in metadata["packages"]:
        if raw["id"] not in workspace_members or raw.get("publish") == []:
            continue
        packages.append(
            Package(
                name=raw["name"],
                version=raw["version"],
                manifest_path=Path(raw["manifest_path"]),
                dependencies=tuple(raw["dependencies"]),
                features=raw["features"],
            )
        )
    return packages


def cargo_toml_string(value: Path) -> str:
    return json.dumps(str(value.resolve()))


def package_archives(
    packages: list[Package], target_directory: Path, allow_dirty: bool
) -> dict[str, Path]:
    package_directories = {package.name: package.directory for package in packages}
    archives = {}

    for package in packages:
        dependency_names = {
            dependency["name"]
            for dependency in package.dependencies
            if dependency["name"] in package_directories
            and dependency["name"] != package.name
        }
        patches = [
            f"patch.crates-io.{name}.path={cargo_toml_string(package_directories[name])}"
            for name in sorted(dependency_names)
        ]
        command = ["cargo", "package", "--package", package.name]
        if allow_dirty:
            command.append("--allow-dirty")
        for patch in patches:
            command.extend(("--config", patch))

        print(f"Packaging {package.name} {package.version}", flush=True)
        subprocess.run(command, check=True)

        archive = target_directory / "package" / f"{package.name}-{package.version}.crate"
        if not archive.is_file():
            raise RuntimeError(f"cargo did not create {archive}")
        archives[package.name] = archive

    return archives


def normalized_manifest(archive: Path, package: Package) -> dict[str, Any]:
    member_name = f"{package.name}-{package.version}/Cargo.toml"
    with tarfile.open(archive, "r:gz") as crate:
        member = crate.extractfile(member_name)
        if member is None:
            raise RuntimeError(f"{archive} does not contain {member_name}")
        return tomllib.loads(member.read().decode())


def dependency_tables(manifest: dict[str, Any]) -> Iterable[dict[str, Any]]:
    for table_name in DEPENDENCY_TABLES:
        yield manifest.get(table_name, {})

    for target in manifest.get("target", {}).values():
        for table_name in DEPENDENCY_TABLES:
            yield target.get(table_name, {})


def published_internal_dependencies(
    manifest: dict[str, Any], package_versions: dict[str, str]
) -> set[str]:
    dependencies = set()
    for table in dependency_tables(manifest):
        for dependency_key, specification in table.items():
            if isinstance(specification, str):
                dependency_name = dependency_key
                requirement = specification
            else:
                dependency_name = specification.get("package", dependency_key)
                requirement = specification.get("version")

            if dependency_name not in package_versions:
                continue

            expected = f"={package_versions[dependency_name]}"
            if requirement != expected:
                raise RuntimeError(
                    f"{manifest['package']['name']} publishes {dependency_name} with "
                    f"version {requirement!r}; expected the exact workspace pin {expected!r}"
                )
            dependencies.add(dependency_name)
    return dependencies


def dependency_is_in_features(package: Package, dependency_name: str) -> bool:
    return any(
        item.split("/", 1)[0] == dependency_name
        for items in package.features.values()
        for item in items
        if "/" in item
    )


def release_plz_order(packages: list[Package]) -> list[str]:
    """Mirror release-plz's normal/build dependency ordering."""
    package_names = {package.name for package in packages}
    packages_by_name = {package.name: package for package in packages}
    order: list[str] = []
    visiting: list[str] = []

    def visit(package: Package) -> None:
        if package.name in order:
            return
        if package.name in visiting:
            cycle = " -> ".join((*visiting, package.name))
            raise RuntimeError(f"release-plz dependency cycle: {cycle}")

        visiting.append(package.name)
        for dependency in package.dependencies:
            dependency_name = dependency["name"]
            kind = dependency.get("kind")
            orders_dependency = kind in (None, "normal", "build") or (
                kind == "dev"
                and dependency_is_in_features(package, dependency_name)
            )
            if (
                orders_dependency
                and dependency_name in package_names
                and dependency_name != package.name
            ):
                visit(packages_by_name[dependency_name])
        visiting.pop()
        order.append(package.name)

    for package in packages:
        visit(package)
    return order


def validate_order(
    packages: list[Package], archives: dict[str, Path], order: list[str]
) -> None:
    package_versions = {package.name: package.version for package in packages}
    packages_by_name = {package.name: package for package in packages}
    dependencies = {}

    for package_name in order:
        package = packages_by_name[package_name]
        manifest = normalized_manifest(archives[package_name], package)
        dependencies[package_name] = published_internal_dependencies(
            manifest, package_versions
        )

    validate_dependency_order(order, dependencies)


def validate_dependency_order(
    order: list[str], dependencies: dict[str, set[str]]
) -> None:
    position = {name: index for index, name in enumerate(order)}
    errors = []

    for package_name in order:
        for dependency_name in sorted(dependencies[package_name]):
            dependency_follows_package = (
                position[dependency_name] > position[package_name]
            )
            if dependency_name != package_name and dependency_follows_package:
                errors.append(
                    f"{package_name} is published before registry dependency "
                    f"{dependency_name}"
                )

    if errors:
        details = "\n  - ".join(errors)
        raise RuntimeError(
            "release-plz would partially publish this workspace:\n"
            f"  - {details}\n"
            "Use a path-only dev-dependency or change the dependency graph."
        )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--allow-dirty",
        action="store_true",
        help="pass --allow-dirty to cargo package for local validation",
    )
    args = parser.parse_args()

    metadata = cargo_metadata()
    packages = publishable_packages(metadata)
    if not packages:
        raise RuntimeError("workspace has no publishable packages")

    order = release_plz_order(packages)
    packages_by_name = {package.name: package for package in packages}
    ordered_packages = [packages_by_name[name] for name in order]
    archives = package_archives(
        ordered_packages, Path(metadata["target_directory"]), args.allow_dirty
    )
    validate_order(packages, archives, order)

    print("Release order is safe:")
    print("  " + " -> ".join(order))


if __name__ == "__main__":
    main()
