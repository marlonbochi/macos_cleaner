import { describe, expect, it } from "vitest";
import {
  isNewer,
  parseVersion,
  readCargoVersion,
  readVersions,
  setCargoVersion,
  setJsonVersion,
  versionProblems,
} from "./version.mjs";

const CARGO = `[workspace]
members = ["crates/*"]

[workspace.package]
version = "0.1.0"
edition = "2024"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
`;

describe("versions", () => {
  it("parses semantic versions with an optional v and pre-release", () => {
    expect(parseVersion("v1.2.3")).toEqual({ major: 1, minor: 2, patch: 3, pre: null });
    expect(parseVersion("0.2.0-beta.1")?.pre).toBe("beta.1");
    for (const bad of ["1.2", "1.2.3.4", "latest", "", "01.a.0"]) {
      expect(parseVersion(bad)).toBeNull();
    }
  });

  it("orders versions, pre-releases before their release", () => {
    expect(isNewer("0.2.0", "0.1.0")).toBe(true);
    expect(isNewer("0.1.10", "0.1.9")).toBe(true);
    expect(isNewer("1.0.0", "0.9.9")).toBe(true);
    expect(isNewer("0.1.0", "0.1.0")).toBe(false);
    expect(isNewer("0.1.0", "0.2.0")).toBe(false);
    expect(isNewer("0.2.0", "0.2.0-beta.1")).toBe(true);
    expect(isNewer("0.2.0-beta.1", "0.2.0")).toBe(false);
    expect(isNewer("0.2.0-beta.10", "0.2.0-beta.2")).toBe(true);
    expect(isNewer("nope", "0.1.0")).toBe(false);
  });

  it("updates only the workspace package version in Cargo.toml", () => {
    const updated = setCargoVersion(CARGO, "0.2.0");
    expect(readCargoVersion(updated)).toBe("0.2.0");
    expect(updated).toContain('serde = { version = "1"');
    expect(() => setCargoVersion('[package]\nversion = "1.0.0"\n', "2.0.0")).toThrow();
  });

  it("updates the version of JSON files", () => {
    const json = setJsonVersion('{"name":"x","version":"0.1.0","private":true}', "0.2.0");
    expect(JSON.parse(json)).toEqual({ name: "x", version: "0.2.0", private: true });
    expect(json.endsWith("\n")).toBe(true);
    expect(() => setJsonVersion("{}", "1.0.0")).toThrow();
  });

  it("reports every file that disagrees with the tag", () => {
    const files = {
      packageJson: '{"version":"0.2.0"}',
      tauriConf: '{"version":"0.1.0"}',
      cargoToml: CARGO,
    };
    const versions = readVersions(files);
    expect(versions).toEqual({ packageJson: "0.2.0", tauriConf: "0.1.0", cargoToml: "0.1.0" });
    expect(versionProblems(versions, "v0.2.0")).toEqual([
      "src-tauri/tauri.conf.json has version 0.1.0, expected 0.2.0",
      "Cargo.toml has version 0.1.0, expected 0.2.0",
    ]);
    expect(
      versionProblems({ packageJson: "0.1.0", tauriConf: "0.1.0", cargoToml: "0.1.0" }, "v0.1.0"),
    ).toEqual([]);
    expect(versionProblems(versions, "latest")[0]).toContain("not a valid version tag");
  });
});
