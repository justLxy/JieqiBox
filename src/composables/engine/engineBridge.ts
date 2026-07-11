/**
 * Engine transport bridge.
 *
 * The chess/jieqi engine is driven over a UCI-like text protocol. On the native
 * (Tauri) build the engine is a real OS process spawned by the Rust backend;
 * commands go out via the `send_to_engine` command and output arrives via the
 * `engine-output` event. On the web build there is no OS process, so the engine
 * runs as a WebAssembly module inside a Web Worker (see `wasmEngine.ts`).
 *
 * This module exposes a single transport-agnostic surface — `spawnEngine`,
 * `sendToEngine`, `killEngine`, and `onEngineOutput` — so the higher-level
 * `useUciEngine` / `useJaiEngine` composables can stay identical across builds.
 * `onEngineOutput` mirrors Tauri's `listen` contract: it resolves to an
 * unsubscribe function.
 *
 * Web transport: the browser cannot launch local programs, so it connects to a
 * small local bridge server (`bridge/server.mjs`) over WebSocket, which spawns
 * the real engine binary and relays its stdin/stdout. See `wsEngine.ts`.
 */
import { isTauri } from '../../utils/runtime'
import type { WsEngineController } from './wsEngine'

export type EngineOutputHandler = (line: string) => void
export type Unlisten = () => void

export interface EngineLineWaiter {
  promise: Promise<void>
  cancel: () => void
}

export interface SpawnOptions {
  /** Native: absolute path to the engine binary. Web: WASM engine identifier. */
  path: string
  /** Native: process arguments. Ignored on web. */
  args: string[]
}

// ---------------------------------------------------------------------------
// Web (WebSocket bridge) transport state
// ---------------------------------------------------------------------------

// Subscribers to engine output. Shared by both transports so the higher-level
// code can attach multiple listeners exactly like the Tauri event bus allowed.
const outputHandlers = new Set<EngineOutputHandler>()

const emitOutput = (line: string) => {
  for (const handler of outputHandlers) {
    try {
      handler(line)
    } catch (e) {
      console.error('[engineBridge] output handler threw:', e)
    }
  }
}

let wsController: WsEngineController | null = null

// ---------------------------------------------------------------------------
// Tauri transport helpers (lazy-imported so the web bundle never loads them)
// ---------------------------------------------------------------------------

let tauriUnlisten: Unlisten | null = null

const ensureTauriListener = async () => {
  if (tauriUnlisten) return
  const { listen } = await import('@tauri-apps/api/event')
  tauriUnlisten = await listen<string>('engine-output', ev => {
    emitOutput(ev.payload)
  })
}

const tauriInvoke = async (cmd: string, args?: Record<string, unknown>) => {
  const { invoke } = await import('@tauri-apps/api/core')
  return invoke(cmd, args)
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Subscribe to engine output lines. Returns an unsubscribe function, matching
 * the Tauri `listen` contract that existing callers already expect.
 */
export const onEngineOutput = async (
  handler: EngineOutputHandler
): Promise<Unlisten> => {
  // On native we must ensure the single underlying Tauri event listener exists;
  // it fans out to all handlers registered here.
  if (isTauri()) {
    await ensureTauriListener()
  }
  outputHandlers.add(handler)
  return () => {
    outputHandlers.delete(handler)
  }
}

/**
 * Register a one-shot protocol waiter before sending the command that produces
 * the response. The listener and timeout are always released once settled.
 */
export const armEngineLineWaiter = async (
  matches: (line: string) => boolean,
  timeoutMs: number,
  timeoutMessage: string
): Promise<EngineLineWaiter> => {
  let unlisten: Unlisten | null = null
  let timeoutId: ReturnType<typeof setTimeout> | null = null
  let settled = false
  let resolvePromise!: () => void
  let rejectPromise!: (reason: Error) => void

  const promise = new Promise<void>((resolve, reject) => {
    resolvePromise = resolve
    rejectPromise = reject
  })

  const cleanup = () => {
    if (timeoutId) {
      clearTimeout(timeoutId)
      timeoutId = null
    }
    unlisten?.()
    unlisten = null
  }

  const resolve = () => {
    if (settled) return
    settled = true
    cleanup()
    resolvePromise()
  }

  const reject = (error: Error) => {
    if (settled) return
    settled = true
    cleanup()
    rejectPromise(error)
  }

  try {
    unlisten = await onEngineOutput(payload => {
      for (const line of payload.split(/\r\n|\n|\r/)) {
        if (matches(line.trim())) {
          resolve()
          return
        }
      }
    })
  } catch (error) {
    reject(error instanceof Error ? error : new Error(String(error)))
  }

  if (settled) {
    unlisten?.()
  } else {
    timeoutId = setTimeout(() => reject(new Error(timeoutMessage)), timeoutMs)
  }

  return {
    promise,
    // Cancellation is used during teardown. Resolve rather than reject so a
    // failed spawn cannot leave an unobserved rejected promise behind.
    cancel: resolve,
  }
}

/** Spawn (native) or instantiate (web) the engine. */
export const spawnEngine = async (opts: SpawnOptions): Promise<void> => {
  if (isTauri()) {
    await ensureTauriListener()
    await tauriInvoke('spawn_engine', { path: opts.path, args: opts.args })
    return
  }

  // Web: connect to the local bridge and have it launch the engine binary.
  const { createWsEngine } = await import('./wsEngine')
  if (wsController) {
    await wsController.terminate()
    wsController = null
  }
  wsController = await createWsEngine(opts.path, opts.args, line =>
    emitOutput(line)
  )
}

/** Send a command line to the engine. */
export const sendToEngine = async (command: string): Promise<void> => {
  if (isTauri()) {
    await tauriInvoke('send_to_engine', { command })
    return
  }
  if (!wsController) {
    throw new Error('Engine not running.')
  }
  wsController.send(command)
}

/** Terminate the running engine. */
export const killEngine = async (): Promise<void> => {
  if (isTauri()) {
    await tauriInvoke('kill_engine')
    return
  }
  if (wsController) {
    await wsController.terminate()
    wsController = null
  }
}
