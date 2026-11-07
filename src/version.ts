/**
 * Single source of truth for the application version.
 * Changing the version string here updates all user-facing dialogs,
 * badges, headers, and exports across the application.
 */
export const APP_VERSION = "0.5.3";
export const APP_NAME = "RAILbench";

export function getAppVersion(): string {
  return APP_VERSION;
}

export function getAppVersionTag(): string {
  return `v${APP_VERSION}`;
}

export function getFullAppTitle(): string {
  return `${APP_NAME} ${getAppVersionTag()}`;
}
