import { describe, it, expect } from "vitest";
import { APP_VERSION, APP_NAME, getAppVersion, getAppVersionTag, getFullAppTitle } from "@/version";
import packageJson from "../package.json";

describe("Application Versioning (Single Source of Truth)", () => {
  it("should have a valid semver string in APP_VERSION", () => {
    expect(APP_VERSION).toMatch(/^\d+\.\d+\.\d+(-[a-zA-Z0-9.]+)?$/);
  });

  it("getAppVersion should return the defined APP_VERSION", () => {
    expect(getAppVersion()).toBe(APP_VERSION);
  });

  it("getAppVersionTag should prefix the version with v", () => {
    expect(getAppVersionTag()).toBe(`v${APP_VERSION}`);
  });

  it("getFullAppTitle should combine APP_NAME and version tag", () => {
    expect(getFullAppTitle()).toBe(`${APP_NAME} v${APP_VERSION}`);
  });

  it("should match the version declared in package.json", () => {
    expect(APP_VERSION).toBe(packageJson.version);
  });
});
