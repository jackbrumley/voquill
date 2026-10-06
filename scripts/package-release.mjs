#!/usr/bin/env node

import { copyFileSync, mkdirSync, existsSync, readdirSync, rmSync } from "node:fs";
import { resolve, dirname, basename } from "node:path";
import { fileURLToPath } from "node:url";
import { ASSET_NAMES, VERSION, assertPlainVersion } from "./release-assets.mjs";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, "..");
const SRC_TAURI = resolve(ROOT, "src-tauri");

const OUT_DIR = resolve(ROOT, "release-artifacts");

function globFirst(dir, pattern) {
  if (!existsSync(dir)) return null;
  const entries = readdirSync(dir);
  const regex = new RegExp(pattern);
  for (const entry of entries) {
    if (regex.test(entry)) return resolve(dir, entry);
  }
  return null;
}

// Tauri embeds the version in the artifact filename (e.g. voquill_1.5.0_amd64.deb).
// Refuse to package an artifact whose embedded version differs from the configured
// release version — otherwise a stale build from a previous release could be
// mislabeled as the current one.
function globFirstForVersion(dir, pattern) {
  if (!existsSync(dir)) return null;
  const entries = readdirSync(dir);
  const regex = new RegExp(pattern);
  for (const entry of entries) {
    if (!regex.test(entry)) continue;
    if (!entry.includes(`_${VERSION}_`) && !entry.includes(`-${VERSION}-`)) {
      console.log(`  SKIP ${entry} (embedded version does not match v${VERSION})`);
      continue;
    }
    return resolve(dir, entry);
  }
  return null;
}

// No checksum files are produced: GitHub computes a SHA-256 digest for every
// uploaded release asset, and the installers verify against that digest.
function copyArtifact(source, targetName) {
  console.log(`  ${basename(source)}  →  ${targetName}`);
  copyFileSync(source, resolve(OUT_DIR, targetName));
}

// Linux releases come only from the containerized build (npm run release:linux),
// never from a host `tauri:build`, so the glibc floor is fixed by the container
// base image rather than by whichever distro the release machine runs.
function packageLinux() {
  const bundleDir = resolve(SRC_TAURI, "target", "linux-release", "bundle");
  console.log("\nPackaging Linux artifacts...\n");

  const deb = globFirstForVersion(resolve(bundleDir, "deb"), /\.deb$/);
  const rpm = globFirstForVersion(resolve(bundleDir, "rpm"), /\.rpm$/);
  const appimage = globFirstForVersion(resolve(bundleDir, "appimage"), /\.AppImage$/);

  let count = 0;
  if (deb) { copyArtifact(deb, ASSET_NAMES.linuxDeb); count++; }
  if (rpm) { copyArtifact(rpm, ASSET_NAMES.linuxRpm); count++; }
  if (appimage) { copyArtifact(appimage, ASSET_NAMES.linuxAppImage); count++; }

  if (count === 0) {
    console.log("  No Linux build artifacts found. Run 'npm run release:linux' first.");
  }
  return count;
}

function packageWindows() {
  const bundleDir = resolve("C:\\vb", "release", "bundle");
  console.log("\nPackaging Windows artifacts...\n");

  const msi = globFirstForVersion(resolve(bundleDir, "msi"), /\.msi$/);
  const nsis = globFirstForVersion(resolve(bundleDir, "nsis"), /\.exe$/);

  let count = 0;
  if (msi) { copyArtifact(msi, ASSET_NAMES.windowsMsi); count++; }
  if (nsis) { copyArtifact(nsis, ASSET_NAMES.windowsSetup); count++; }

  if (count === 0) {
    console.log("  No Windows build artifacts found. Run 'npm run tauri:build' first.");
  }
  return count;
}

function main() {
  assertPlainVersion();
  // Start from an empty folder so its whole contents can be uploaded as-is.
  rmSync(OUT_DIR, { recursive: true, force: true });
  mkdirSync(OUT_DIR, { recursive: true });

  console.log(`Voquill v${VERSION} release packaging`);
  console.log(`Output: ${OUT_DIR}`);

  let total = 0;

  if (process.platform === "win32") {
    total += packageWindows();
  } else {
    total += packageLinux();
  }

  console.log(`\nDone. ${total} artifact(s) packaged in ${OUT_DIR}`);
  if (total === 0) {
    const buildCommand = process.platform === "win32" ? "npm run tauri:build" : "npm run release:linux";
    console.log(`Nothing to do — build the app first with: ${buildCommand}`);
    process.exit(1);
  }
}

main();