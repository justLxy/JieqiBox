/**
 * Platform I/O helpers: clipboard, file download, and file open.
 *
 * On the native (Tauri) build these delegate to Rust commands and native
 * dialogs. On the web build they use browser primitives:
 *   - clipboard   -> navigator.clipboard
 *   - file save   -> Blob + <a download>
 *   - file open   -> <input type="file">
 */
import { isTauri } from './runtime'

/** Copy text to the clipboard. */
export const copyText = async (text: string): Promise<void> => {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core')
    await invoke('copy_to_clipboard', { text })
    return
  }
  await navigator.clipboard.writeText(text)
}

/**
 * Save a text file. On web this triggers a browser download and returns the
 * filename. On native the caller should keep using the Tauri commands directly
 * where a save dialog / Android path is required.
 */
export const downloadTextFile = (
  content: string,
  filename: string,
  mime = 'application/json'
): string => {
  const blob = new Blob([content], { type: `${mime};charset=utf-8` })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = filename
  document.body.appendChild(a)
  a.click()
  document.body.removeChild(a)
  // Revoke on the next tick so the download has a chance to start.
  setTimeout(() => URL.revokeObjectURL(url), 0)
  return filename
}

/**
 * Prompt the user to pick a file and return its text contents, or null if the
 * selection was cancelled. Web-only helper; native code paths use the Tauri
 * dialog plugin.
 */
export const pickTextFile = (accept = ''): Promise<string | null> => {
  return new Promise(resolve => {
    const input = document.createElement('input')
    input.type = 'file'
    if (accept) input.accept = accept
    input.onchange = () => {
      const file = input.files?.[0]
      if (!file) {
        resolve(null)
        return
      }
      const reader = new FileReader()
      reader.onload = () => resolve(String(reader.result ?? ''))
      reader.onerror = () => resolve(null)
      reader.readAsText(file)
    }
    // If the dialog is dismissed without choosing, there is no reliable event;
    // callers treat a missing result as a cancel.
    input.click()
  })
}
