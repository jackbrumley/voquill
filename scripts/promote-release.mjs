#!/usr/bin/env node

// Final release step: publishes the pre-release for the version in
// tauri.conf.json as the latest release and fast-forwards `main` to the
// release tag, which also deploys the website and install scripts (GitHub
// Pages serves main:/docs). Refuses while assets are missing or the notes
// still contain TODO lines.
//
//   npm run release:promote
//
// Agents: never run this unless the maintainer explicitly asks.

import { execFileSync } from "node:child_process";
import { TAG, assertPlainVersion } from "./release-assets.mjs";
import { fetchRelease, findAssetProblems } from "./verify-release.mjs";

function fail(message) {
  console.error(`\nRelease promotion failed: ${message}`);
  process.exit(1);
}

function run(command, args) {
  console.log(`$ ${command} ${args.join(" ")}`);
  execFileSync(command, args, { stdio: "inherit" });
}

assertPlainVersion();

const release = fetchRelease(TAG);
if (!release.prerelease) {
  fail(`${TAG} is already published`);
}

const problems = findAssetProblems(release);
if (/\bTODO\b/.test(release.body ?? "")) {
  problems.push("release notes still contain TODO lines");
}
if (problems.length > 0) {
  fail(`${TAG} is not ready:\n${problems.map((problem) => `  - ${problem}`).join("\n")}`);
}

run("gh", ["release", "edit", TAG, "--prerelease=false", "--latest"]);
// A plain (non-force) push only succeeds as a fast-forward, so main can never
// lose commits here.
run("git", ["push", "origin", `${TAG}^{commit}:refs/heads/main`]);

console.log(`\n${TAG} is published as the latest release, and main now points at it.`);
