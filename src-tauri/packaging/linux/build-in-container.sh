#!/usr/bin/env bash
# Runs inside the Linux release container (see Containerfile).
#   /src   - the host repository, mounted read-only
#   /out   - receives the deb, rpm and AppImage bundles
#   /cache - persistent volume for Cargo registry, build target and npm cache
set -euo pipefail

cargo_registry_cache=/cache/cargo-registry
export CARGO_TARGET_DIR=/cache/target
export npm_config_cache=/cache/npm
mkdir -p "$cargo_registry_cache" "$CARGO_TARGET_DIR" "$npm_config_cache"
ln -sfn "$cargo_registry_cache" "${CARGO_HOME}/registry"

# Bundles from earlier runs persist in the target volume; clear them so only
# this build's artifacts are copied out.
bundle_dir="${CARGO_TARGET_DIR}/release/bundle"
rm -rf "$bundle_dir"

# Build from a fresh copy of the working tree: tracked plus untracked,
# non-ignored files only, so host node_modules/target never leak in.
git config --global --add safe.directory /src
rm -rf /build
mkdir -p /build
git -C /src ls-files -z --cached --others --exclude-standard \
    | rsync -a --from0 --files-from=- --ignore-missing-args /src/ /build/

cd /build
npm ci
npm run tauri:build

for bundle in deb rpm appimage; do
    rm -rf "/out/${bundle}"
    mkdir -p "/out/${bundle}"
done
cp "${bundle_dir}"/deb/*.deb /out/deb/
cp "${bundle_dir}"/rpm/*.rpm /out/rpm/
cp "${bundle_dir}"/appimage/*.AppImage /out/appimage/

echo "Linux release bundles written to /out (glibc $(ldd --version | head -n1 | awk '{print $NF}'))"
