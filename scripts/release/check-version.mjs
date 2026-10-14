#!/usr/bin/env node
// Fails when the app version in package.json, tauri.conf.json and Cargo.toml does not match the
// given tag. Used by the release workflow: `node scripts/release/check-version.mjs v0.2.0`.
import { readFileSync } from "node:fs";
import { VERSION_FILES, readVersions, versionProblems } from "./version.mjs";

const tag = process.argv[2];
if (!tag) {
  console.error("usage: check-version.mjs <tag>");
  process.exit(2);
}
const contents = Object.fromEntries(
  Object.entries(VERSION_FILES).map(([key, file]) => [key, readFileSync(file, "utf8")]),
);
const problems = versionProblems(readVersions(contents), tag);
if (problems.length > 0) {
  for (const problem of problems) console.error(`✗ ${problem}`);
  process.exit(1);
}
console.log(`✓ every version file says ${tag.replace(/^v/, "")}`);
