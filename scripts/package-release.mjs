#!/usr/bin/env node

import { readFileSync, copyFileSync, mkdirSync, existsSync, writeFileSync, readdirSync } from "node:fs";
import { createHash } from "node:crypto";
import { resolve, dirname, basename } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, "..");
const SRC_TAURI = resolve(ROOT, "src-tauri");

const TAURI_CONF = JSON.parse(readFileSync(resolve(SRC_TAURI, "tauri.conf.json"), "utf8"));
const VERSION = TAURI_CONF.version;
const OUT_DIR = resolve(ROOT, "release-artifacts");

function sha256(filePath) {
  const content = readFileSync(filePath);
  return createHash("sha256").update(content).digest("hex");
}

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

function copyAndChecksum(source, targetName) {
  const targetPath = resolve(OUT_DIR, targetName);
  console.log(`  ${basename(source)}  →  ${targetName}`);
  copyFileSync(source, targetPath);

  const hash = sha256(targetPath);
  const checksumPath = `${targetPath}.sha256`;
  writeFileSync(checksumPath, `${hash}  ${targetName}\n`);
  console.log(`  ${targetName}.sha256  ✓`);
}

function packageLinux() {
  const bundleDir = resolve(SRC_TAURI, "target", "release", "bundle");
  console.log("\nPackaging Linux artifacts...\n");

  const deb = globFirstForVersion(resolve(bundleDir, "deb"), /\.deb$/);
  const rpm = globFirstForVersion(resolve(bundleDir, "rpm"), /\.rpm$/);
  const appimage = globFirstForVersion(resolve(bundleDir, "appimage"), /\.AppImage$/);

  let count = 0;
  if (deb) { copyAndChecksum(deb, `voquill-${VERSION}-linux-x64.deb`); count++; }
  if (rpm) { copyAndChecksum(rpm, `voquill-${VERSION}-linux-x64.rpm`); count++; }
  if (appimage) { copyAndChecksum(appimage, `voquill-${VERSION}-linux-x64.AppImage`); count++; }

  if (count === 0) {
    console.log("  No Linux build artifacts found. Run 'npm run tauri:build' first.");
  }
  return count;
}

function packageWindows() {
  const bundleDir = resolve("C:\\vb", "release", "bundle");
  console.log("\nPackaging Windows artifacts...\n");

  const msi = globFirstForVersion(resolve(bundleDir, "msi"), /\.msi$/);
  const nsis = globFirstForVersion(resolve(bundleDir, "nsis"), /\.exe$/);

  let count = 0;
  if (msi) { copyAndChecksum(msi, `voquill-${VERSION}-windows-x64.msi`); count++; }
  if (nsis) { copyAndChecksum(nsis, `voquill-${VERSION}-windows-x64-setup.exe`); count++; }

  if (count === 0) {
    console.log("  No Windows build artifacts found. Run 'npm run tauri:build' first.");
  }
  return count;
}

function main() {
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
    console.log("Nothing to do — build the app first with: npm run tauri:build");
    process.exit(1);
  }
}

main();