// Version helpers shared by the release scripts. Pure functions: tested in version.test.mjs.

const SEMVER = /^(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z.-]+))?$/;

/** Files that carry the app version; they must always agree. */
export const VERSION_FILES = {
  packageJson: "package.json",
  tauriConf: "src-tauri/tauri.conf.json",
  cargoToml: "Cargo.toml",
};

/** Parses "1.2.3" or "1.2.3-beta.1" (a leading "v" is accepted). Returns null when invalid. */
export function parseVersion(text) {
  const match = SEMVER.exec(String(text).trim().replace(/^v/, ""));
  if (!match) return null;
  const [, major, minor, patch, pre] = match;
  return { major: Number(major), minor: Number(minor), patch: Number(patch), pre: pre ?? null };
}

/** Whether `next` is a higher version than `current` (a pre-release sorts before its release). */
export function isNewer(next, current) {
  const a = parseVersion(next);
  const b = parseVersion(current);
  if (!a || !b) return false;
  for (const key of ["major", "minor", "patch"]) {
    if (a[key] !== b[key]) return a[key] > b[key];
  }
  if (a.pre === b.pre) return false;
  if (a.pre === null) return true;
  if (b.pre === null) return false;
  return a.pre.localeCompare(b.pre, "en", { numeric: true }) > 0;
}

/** Version declared in `[workspace.package]` of the root Cargo.toml. */
export function readCargoVersion(toml) {
  const section = /\[workspace\.package\][^[]*?\nversion\s*=\s*"([^"]+)"/.exec(toml);
  return section ? section[1] : null;
}

export function setCargoVersion(toml, version) {
  if (readCargoVersion(toml) === null) throw new Error("no [workspace.package] version found");
  return toml.replace(/(\[workspace\.package\][^[]*?\nversion\s*=\s*")[^"]+(")/, `$1${version}$2`);
}

/** Updates the top-level "version" of a JSON document, keeping its formatting style. */
export function setJsonVersion(json, version) {
  const data = JSON.parse(json);
  if (typeof data.version !== "string") throw new Error("no top-level version found");
  data.version = version;
  return `${JSON.stringify(data, null, 2)}\n`;
}

/** Versions found in each file's contents, keyed like VERSION_FILES. */
export function readVersions({ packageJson, tauriConf, cargoToml }) {
  return {
    packageJson: JSON.parse(packageJson).version ?? null,
    tauriConf: JSON.parse(tauriConf).version ?? null,
    cargoToml: readCargoVersion(cargoToml),
  };
}

/** Problems that would make a release for `tag` inconsistent; empty when everything agrees. */
export function versionProblems(versions, tag) {
  const expected = String(tag).replace(/^v/, "");
  const problems = [];
  if (!parseVersion(expected)) problems.push(`"${tag}" is not a valid version tag (vX.Y.Z)`);
  for (const [file, found] of Object.entries(versions)) {
    if (found !== expected) {
      problems.push(
        `${VERSION_FILES[file]} has version ${found ?? "(none)"}, expected ${expected}`,
      );
    }
  }
  return problems;
}
