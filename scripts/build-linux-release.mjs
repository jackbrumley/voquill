#!/usr/bin/env node

// Builds the Linux release bundles (deb, rpm, AppImage) inside the Ubuntu
// 22.04 container defined in src-tauri/packaging/linux/Containerfile, so the
// glibc floor of every Linux artifact is fixed regardless of the host distro.
// Output lands in src-tauri/target/linux-release/bundle, which is the only
// Linux source scripts/package-release.mjs packages from.

import { execFileSync, spawnSync } from "node:child_process";
import { mkdirSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const CONTAINER_CONTEXT = resolve(ROOT, "src-tauri", "packaging", "linux");
const OUTPUT_DIR = resolve(ROOT, "src-tauri", "target", "linux-release", "bundle");
const IMAGE_TAG = "voquill-linux-release:ubuntu22.04";
const CACHE_VOLUME = "voquill-linux-release-cache";

function fail(message) {
  console.error(`\nLinux release build failed: ${message}`);
  process.exit(1);
}

function podman(args) {
  console.log(`\n$ podman ${args.join(" ")}\n`);
  try {
    execFileSync("podman", args, { stdio: "inherit" });
  } catch {
    fail(`'podman ${args[0]}' exited with an error (see output above).`);
  }
}

if (process.platform !== "linux") {
  fail("Linux release bundles can only be built on a Linux host.");
}

if (spawnSync("podman", ["--version"], { stdio: "ignore" }).status !== 0) {
  fail("podman is required (Fedora: 'sudo dnf install podman', Debian/Ubuntu/Mint: 'sudo apt install podman').");
}

mkdirSync(OUTPUT_DIR, { recursive: true });

podman(["build", "--tag", IMAGE_TAG, "--file", resolve(CONTAINER_CONTEXT, "Containerfile"), CONTAINER_CONTEXT]);

// label=disable avoids SELinux relabeling the whole repository for the
// read-only source mount; rootless podman maps container root to the host user,
// so the bundles in OUTPUT_DIR are owned by the invoking user.
podman([
  "run",
  "--rm",
  "--security-opt", "label=disable",
  "--volume", `${ROOT}:/src:ro`,
  "--volume", `${OUTPUT_DIR}:/out`,
  "--volume", `${CACHE_VOLUME}:/cache`,
  IMAGE_TAG,
]);

console.log(`\nLinux release bundles: ${OUTPUT_DIR}`);
console.log("Next: node scripts/package-release.mjs");
