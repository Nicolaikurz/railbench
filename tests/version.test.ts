import { describe, it, expect } from "vitest";
import { APP_VERSION, APP_NAME, formatCalVer, getAppVersion, getAppVersionTag, getFullAppTitle } from "@/version";
import packageJson from "../package.json";

describe("Application Versioning (Single Source of Truth)", () => {
  it("should have a valid CalVer YY.MM.PATCH string in APP_VERSION", () => {
    expect(APP_VERSION).toMatch(/^\d{2}\.(0[1-9]|1[0-2])\.\d+(-[a-zA-Z0-9.]+)?$/);
  });

  it("formatCalVer should format build year and month cleanly", () => {
    expect(formatCalVer(new Date(Date.UTC(2026, 11, 15)), 1)).toBe("26.12.1");
    expect(formatCalVer(new Date(Date.UTC(2026, 10, 25)), 1)).toBe("26.11.1");
    expect(formatCalVer(new Date(Date.UTC(2027, 0, 5)), 2)).toBe("27.01.2");
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
