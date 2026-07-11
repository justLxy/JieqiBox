/**
 * WebSocket engine controller (web build).
 *
 * The browser cannot launch local programs, so the web build talks to a small
 * local bridge server (see `bridge/server.mjs`) over WebSocket. The bridge
 * spawns the real engine binary on your machine and relays its UCI/JAI protocol
 * back and forth. This keeps full engine functionality (any native engine,
 * NNUE files, multi-threading) without compiling anything to WASM.
 *
 * The controller mirrors the shape used by the native transport: `send` writes
 * a command line, `terminate` stops the engine. Engine output is delivered via
 * the `onLine` callback passed to `createWsEngine`.
 */

export interface WsEngineController {
  /** Send a UCI/JAI command line to the engine. */
  send: (command: string) => void
  /** Terminate the engine and close the socket. */
  terminate: () => Promise<void>
}

/** Default bridge endpoint. Overridable via localStorage key `jieqibox.bridgeUrl`. */
export const getBridgeUrl = (): string => {
  try {
    const stored = localStorage.getItem('jieqibox.bridgeUrl')
    if (stored && stored.trim()) return stored.trim()
  } catch {
    /* ignore */
  }
  return 'ws://127.0.0.1:8181'
}

interface BridgeMessage {
  type: 'spawned' | 'output' | 'exit' | 'error'
  line?: string
  code?: number
  message?: string
}

/**
 * Connect to the bridge, spawn the engine at `enginePath`, and resolve with a
 * controller once the engine process has started.
 *
 * @param enginePath Absolute path to the engine binary on the machine running
 *                   the bridge.
 * @param args       Command-line arguments for the engine.
 * @param onLine     Called for every line the engine emits.
 */
export const createWsEngine = async (
  enginePath: string,
  args: string[],
  onLine: (line: string) => void
): Promise<WsEngineController> => {
  const url = getBridgeUrl()
  const ws = new WebSocket(url)

  await new Promise<void>((resolve, reject) => {
    const timeout = setTimeout(() => {
      reject(
        new Error(
          `Could not reach the engine bridge at ${url}. Start it with "npm run bridge".`
        )
      )
    }, 5000)

    ws.onopen = () => {
      // Ask the bridge to launch the engine.
      ws.send(JSON.stringify({ type: 'spawn', path: enginePath, args }))
    }

    ws.onerror = () => {
      clearTimeout(timeout)
      reject(
        new Error(
          `Failed to connect to the engine bridge at ${url}. Is it running? Start it with "npm run bridge".`
        )
      )
    }

    ws.onmessage = ev => {
      let msg: BridgeMessage
      try {
        msg = JSON.parse(ev.data)
      } catch {
        return
      }
      if (msg.type === 'spawned') {
        clearTimeout(timeout)
        resolve()
      } else if (msg.type === 'error') {
        clearTimeout(timeout)
        reject(new Error(msg.message || 'Engine bridge error'))
      } else if (msg.type === 'output' && typeof msg.line === 'string') {
        // Output can arrive before this promise resolves; forward it anyway.
        onLine(msg.line)
      }
    }
  })

  // Steady-state message handling (replaces the init handler above).
  ws.onmessage = ev => {
    let msg: BridgeMessage
    try {
      msg = JSON.parse(ev.data)
    } catch {
      return
    }
    if (msg.type === 'output' && typeof msg.line === 'string') {
      onLine(msg.line)
    } else if (msg.type === 'error') {
      console.error('[wsEngine] bridge error:', msg.message)
    } else if (msg.type === 'exit') {
      console.log(`[wsEngine] engine exited with code ${msg.code}`)
    }
  }

  return {
    send: (command: string) => {
      if (ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({ type: 'cmd', command }))
      }
    },
    terminate: async () => {
      if (ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({ type: 'kill' }))
      }
      ws.close()
    },
  }
}
