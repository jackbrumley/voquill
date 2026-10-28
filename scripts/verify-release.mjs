#!/usr/bin/env node

// Checks that the GitHub release for the version in tauri.conf.json carries
// exactly the assets in EXPECTED_ASSETS, each with the SHA-256 digest the
// installers verify against. The release workflow runs this after all
// platform builds upload, and promote-release.mjs runs it before publishing.
//
//   node scripts/verify-release.mjs

import { execFileSync } from "node:child_process";
import { pathToFileURL } from "node:url";
import { EXPECTED_ASSETS, REPOSITORY, TAG } from "./release-assets.mjs";

// Reads the release through the REST API, the same source (and `digest`
// field) install.sh and install.ps1 use.
export function fetchRelease(tag) {
  const json = execFileSync("gh", ["api", `repos/${REPOSITORY}/releases/tags/${tag}`], { encoding: "utf8" });
  return JSON.parse(json);
}

// Returns a list of problems; empty means the release is complete.
export function findAssetProblems(release) {
  const published = new Map(release.assets.map((asset) => [asset.name, asset]));
  const problems = [];

  for (const name of EXPECTED_ASSETS) {
    const asset = published.get(name);
    if (!asset) {
      problems.push(`missing asset: ${name}`);
    } else if (!asset.digest) {
      problems.push(`asset has no digest yet: ${name}`);
    }
  }
  for (const name of published.keys()) {
    if (!EXPECTED_ASSETS.includes(name)) {
      problems.push(`unexpected asset: ${name}`);
    }
  }
  return problems;
}

function main() {
  const problems = findAssetProblems(fetchRelease(TAG));
  if (problems.length > 0) {
    console.error(`Release ${TAG} is incomplete:\n${problems.map((problem) => `  - ${problem}`).join("\n")}`);
    process.exit(1);
  }
  console.log(`Release ${TAG} carries all ${EXPECTED_ASSETS.length} expected assets:`);
  for (const name of EXPECTED_ASSETS) {
    console.log(`  ${name}`);
  }
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  main();
}
