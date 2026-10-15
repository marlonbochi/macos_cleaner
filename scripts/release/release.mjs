#!/usr/bin/env node
// Prepares a release: bumps the version everywhere, regenerates CHANGELOG.md, commits and tags.
// Pushing the tag starts the GitHub workflow that builds the app and publishes the release.
//
//   npm run release -- 0.2.0            # prepare commit + tag locally
//   npm run release -- 0.2.0 --dry-run  # only show what would change
import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import {
  VERSION_FILES,
  isNewer,
  parseVersion,
  readVersions,
  setCargoVersion,
  setJsonVersion,
} from "./version.mjs";

const GIT_CLIFF = "git-cliff@2.14.2";

const run = (cmd, args, options = {}) =>
  execFileSync(cmd, args, { encoding: "utf8", stdio: ["ignore", "pipe", "inherit"], ...options });

function fail(message) {
  console.error(`✗ ${message}`);
  process.exit(1);
}

const args = process.argv.slice(2);
const dryRun = args.includes("--dry-run");
const version = args.find((a) => !a.startsWith("--"))?.replace(/^v/, "");
if (!version || !parseVersion(version)) fail("usage: npm run release -- <x.y.z> [--dry-run]");
const tag = `v${version}`;

const read = (file) => readFileSync(file, "utf8");
const current = readVersions({
  packageJson: read(VERSION_FILES.packageJson),
  tauriConf: read(VERSION_FILES.tauriConf),
  cargoToml: read(VERSION_FILES.cargoToml),
});
const firstRelease = run("git", ["tag", "--list", "v*"]).trim() === "";
// The very first release may keep the version the project started with.
if (!isNewer(version, current.packageJson) && !(firstRelease && version === current.packageJson)) {
  fail(`${version} is not newer than the current version ${current.packageJson}`);
}
if (run("git", ["branch", "--show-current"]).trim() !== "main") fail("releases are cut from main");
if (run("git", ["status", "--porcelain"]).trim() !== "") fail("the working tree is not clean");
if (run("git", ["tag", "--list", tag]).trim() !== "") fail(`tag ${tag} already exists`);

console.log(`Preparing ${tag} (current ${current.packageJson})${dryRun ? " — dry run" : ""}`);
if (dryRun) {
  console.log(run("npx", ["-y", GIT_CLIFF, "--unreleased", "--tag", tag, "--strip", "all"]));
  process.exit(0);
}

// `npm version` updates package.json and package-lock.json without touching anything else.
run("npm", ["version", version, "--no-git-tag-version", "--allow-same-version"]);
writeFileSync(VERSION_FILES.tauriConf, setJsonVersion(read(VERSION_FILES.tauriConf), version));
writeFileSync(VERSION_FILES.cargoToml, setCargoVersion(read(VERSION_FILES.cargoToml), version));
run("cargo", ["update", "--workspace"]);
run("npx", ["-y", GIT_CLIFF, "--tag", tag, "--output", "CHANGELOG.md"]);
run("npx", ["prettier", "--write", "CHANGELOG.md", VERSION_FILES.tauriConf]);

run("git", ["add", "-A"]);
run("git", ["commit", "-m", `chore(release): ${tag}`]);
run("git", ["tag", "-a", tag, "-m", `Disk Cleaner ${tag}`]);
console.log(`✓ committed and tagged ${tag}. Publish it with:\n\n  git push origin main ${tag}\n`);
