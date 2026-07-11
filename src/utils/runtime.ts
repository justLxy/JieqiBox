/**
 * Runtime environment detection and platform abstraction.
 *
 * The application ships both as a Tauri native app (desktop / Android) and as a
 * plain web page. Native-only capabilities (spawning engine processes, reading
 * and writing config files, native dialogs, clipboard) are provided by the
 * Tauri backend and are unavailable in a browser. This module centralizes the
 * environment check so callers can branch on it and pick the appropriate
 * provider (see `engine/` and `storage.ts`).
 */

/**
 * Returns true when running inside a Tauri webview (desktop or Android), false
 * when running as a plain web page in a normal browser.
 *
 * Detection relies on the `__TAURI_INTERNALS__` object that the Tauri runtime
 * injects into the webview. `__TAURI__` is only present when
 * `app.withGlobalTauri` is enabled, so it is not reliable on its own.
 */
export const isTauri = (): boolean => {
  if (typeof window === 'undefined') return false
  const w = window as any
  return (
    typeof w.__TAURI_INTERNALS__ !== 'undefined' ||
    typeof w.__TAURI__ !== 'undefined'
  )
}

/**
 * Convenience inverse of {@link isTauri} for readability at call sites that
 * branch specifically on the web build.
 */
export const isWeb = (): boolean => !isTauri()
