#!/bin/sh
set -eu
repo_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_dir"
version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n 1)
archive_name="pacmanager-v$version-linux-x86_64"
stage_dir="target/dist/$archive_name"
mkdir -p "$stage_dir"
install -m755 target/release/pacmanager "$stage_dir/pacmanager"
install -m755 install.sh "$stage_dir/install.sh"
cp README.md LICENSE CHANGELOG.md "$stage_dir/"
cp -R assets docs "$stage_dir/"
tar -czf "target/dist/$archive_name.tar.gz" -C target/dist "$archive_name"
(cd target/dist && sha256sum "$archive_name.tar.gz" > SHA256SUMS)
echo "target/dist/$archive_name.tar.gz"
