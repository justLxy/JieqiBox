/**
 * Persistent string storage abstraction.
 *
 * On the native (Tauri) build these map to Rust commands that read/write files
 * in the app data directory. On the web build they fall back to `localStorage`,
 * keyed so the config and autosave blobs stay separate.
 *
 * The stored values are opaque strings (INI for config, JSON for autosave),
 * matching the existing Tauri command contracts, so the higher-level
 * composables need no changes beyond routing through this module.
 */
import { isTauri } from './runtime'

const CONFIG_KEY = 'jieqibox.config'
const AUTOSAVE_KEY = 'jieqibox.autosave'

// Lazily import the Tauri core only when actually running under Tauri, so the
// web bundle never evaluates it.
const tauriInvoke = async <T>(
  cmd: string,
  args?: Record<string, unknown>
): Promise<T> => {
  const { invoke } = await import('@tauri-apps/api/core')
  return invoke<T>(cmd, args)
}

/** Load the config blob (INI). Returns empty string when nothing is stored. */
export const loadConfigString = async (): Promise<string> => {
  if (isTauri()) {
    return (await tauriInvoke<string>('load_config')) || ''
  }
  return localStorage.getItem(CONFIG_KEY) || ''
}

/** Persist the config blob (INI). */
export const saveConfigString = async (content: string): Promise<void> => {
  if (isTauri()) {
    await tauriInvoke('save_config', { content })
    return
  }
  localStorage.setItem(CONFIG_KEY, content)
}

/** Remove all persisted config. */
export const clearConfigString = async (): Promise<void> => {
  if (isTauri()) {
    await tauriInvoke('clear_config')
    return
  }
  localStorage.removeItem(CONFIG_KEY)
}

/** Persist the autosave blob (JSON). */
export const saveAutosaveString = async (content: string): Promise<void> => {
  if (isTauri()) {
    await tauriInvoke('save_autosave', { content })
    return
  }
  localStorage.setItem(AUTOSAVE_KEY, content)
}

/** Load the autosave blob (JSON). Returns empty string when nothing is stored. */
export const loadAutosaveString = async (): Promise<string> => {
  if (isTauri()) {
    return (await tauriInvoke<string>('load_autosave')) || ''
  }
  return localStorage.getItem(AUTOSAVE_KEY) || ''
}
