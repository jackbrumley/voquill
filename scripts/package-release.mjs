#!/usr/bin/env node

import { readFileSync, copyFileSync, mkdirSync, existsSync, readdirSync, rmSync } from "node:fs";
import { resolve, dirname, basename } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, "..");
const SRC_TAURI = resolve(ROOT, "src-tauri");

const TAURI_CONF = JSON.parse(readFileSync(resolve(SRC_TAURI, "tauri.conf.json"), "utf8"));
const VERSION = TAURI_CONF.version;
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
  if (deb) { copyArtifact(deb, `voquill-${VERSION}-linux-x64.deb`); count++; }
  if (rpm) { copyArtifact(rpm, `voquill-${VERSION}-linux-x64.rpm`); count++; }
  if (appimage) { copyArtifact(appimage, `voquill-${VERSION}-linux-x64.AppImage`); count++; }

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
  if (msi) { copyArtifact(msi, `voquill-${VERSION}-windows-x64.msi`); count++; }
  if (nsis) { copyArtifact(nsis, `voquill-${VERSION}-windows-x64-setup.exe`); count++; }

  if (count === 0) {
    console.log("  No Windows build artifacts found. Run 'npm run tauri:build' first.");
  }
  return count;
}

// Asset names are `voquill-<version>-<os>-<arch>[-setup].<ext>`. A plain
// MAJOR.MINOR.PATCH version keeps them unambiguous (a suffix like `-beta.1`
// would add hyphens that collide with the separators).
function assertPlainVersion() {
  if (!/^\d+\.\d+\.\d+$/.test(VERSION)) {
    console.error(`Release version '${VERSION}' must be plain MAJOR.MINOR.PATCH (no pre-release or build suffix).`);
    process.exit(1);
  }
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