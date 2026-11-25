declare const __RAILBENCH_BUILD_VERSION__: string | undefined;

/**
 * Single source of truth for the application version (CalVer YY.MM.PATCH).
 * During production builds, `__RAILBENCH_BUILD_VERSION__` stamps the build year and month.
 */
export const APP_VERSION: string =
  typeof __RAILBENCH_BUILD_VERSION__ === "string" && __RAILBENCH_BUILD_VERSION__.length > 0
    ? __RAILBENCH_BUILD_VERSION__
    : "26.11.1";
export const APP_NAME = "RAILbench";

export function formatCalVer(date: Date, patch = 1): string {
  const yy = String(date.getUTCFullYear() % 100).padStart(2, "0");
  const mm = String(date.getUTCMonth() + 1).padStart(2, "0");
  return `${yy}.${mm}.${patch}`;
}

export function getAppVersion(): string {
  return APP_VERSION;
}

export function getAppVersionTag(): string {
  return `v${APP_VERSION}`;
}

export function getFullAppTitle(): string {
  return `${APP_NAME} ${getAppVersionTag()}`;
}
