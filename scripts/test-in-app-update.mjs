#!/usr/bin/env node

// Checks the in-app updater on an installed build: runs it with the hidden
// --update flag, which takes the same path as Update Now (start the update
// script, wait for its started line, exit), then waits for the update script
// to finish and checks that it succeeded. The release workflow runs this
// after installing on each platform, with VOQUILL_UPDATE_BASE_URL pointing at
// the tag's docs/ so the tag's own update and install scripts are used.
//
// This is the test that would have caught the Windows updater that closed
// Voquill without updating: it exercises the app's own process launch, not
// just the scripts.
//
//   node scripts/test-in-app-update.mjs <path to the installed voquill binary>

import { spawn } from "node:child_process";
import { readFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

// Startup takes 20s+ on a fresh runner; the update script then has up to 30s
// to report that it started.
const APP_EXIT_TIMEOUT_MS = 180_000;
// Downloading and installing the release.
const UPDATE_FINISH_TIMEOUT_MS = 600_000;
const STARTED_LINE = "[voquill-update] started";
const UPDATED_LINE = "[voquill-update] finished: updated";
const FAILED_LINE = "[voquill-update] finished: failed";
const DEBUG_DIR = join(homedir(), ".config", "voquill-app", "debug");
const UPDATE_LOG = join(DEBUG_DIR, "update.log");
const SESSION_LOG = join(DEBUG_DIR, "session.log");

const binary = process.argv[2];
if (!binary) {
  console.error("Usage: node scripts/test-in-app-update.mjs <voquill binary>");
  process.exit(2);
}
if (!process.env.VOQUILL_UPDATE_BASE_URL) {
  console.error("VOQUILL_UPDATE_BASE_URL must point at the scripts under test");
  process.exit(2);
}

const sleep = (milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds));

function read(path) {
  try {
    return readFileSync(path, "utf8");
  } catch {
    return "";
  }
}

function fail(message) {
  console.error(`FAIL: ${message}`);
  console.error(`--- ${UPDATE_LOG} ---\n${read(UPDATE_LOG)}`);
  console.error(`--- ${SESSION_LOG} ---\n${read(SESSION_LOG)}`);
  process.exit(1);
}

const app = spawn(binary, ["--update"], { stdio: "ignore" });
const appExit = await Promise.race([
  new Promise((resolve) => app.on("exit", (code, signal) => resolve({ code, signal }))),
  sleep(APP_EXIT_TIMEOUT_MS).then(() => null),
]);
if (!appExit) {
  app.kill();
  fail(`voquill --update did not exit within ${APP_EXIT_TIMEOUT_MS / 1000}s`);
}
if (appExit.code !== 0) {
  fail(`voquill --update exited with ${JSON.stringify(appExit)} (the updater did not start)`);
}
if (!read(UPDATE_LOG).includes(STARTED_LINE)) {
  fail("voquill --update exited, but the update log has no started line");
}
console.log("Updater started; waiting for the update to finish");

const deadline = Date.now() + UPDATE_FINISH_TIMEOUT_MS;
while (Date.now() < deadline) {
  const log = read(UPDATE_LOG);
  if (log.includes(UPDATED_LINE)) {
    console.log(`--- ${UPDATE_LOG} ---\n${log}`);
    console.log("PASS: the in-app updater started the update script and the update succeeded");
    process.exit(0);
  }
  if (log.includes(FAILED_LINE)) {
    fail("the update script reported a failed update");
  }
  await sleep(2_000);
}
fail(`the update did not finish within ${UPDATE_FINISH_TIMEOUT_MS / 1000}s`);
