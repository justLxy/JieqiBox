/**
 * JieqiBox local engine bridge.
 *
 * The web build of JieqiBox runs in a browser sandbox and cannot launch local
 * programs. This small WebSocket server runs on your own machine, launches the
 * engine binary you point it at, and relays the UCI/JAI text protocol between
 * the engine's stdin/stdout and the browser.
 *
 * Protocol (JSON messages over WebSocket):
 *   browser -> bridge:
 *     { "type": "spawn", "path": "/abs/path/to/engine", "args": ["..."] }
 *     { "type": "cmd",   "command": "go depth 20" }
 *     { "type": "kill" }
 *   bridge -> browser:
 *     { "type": "spawned" }              engine process started
 *     { "type": "output", "line": "…" }  one line from engine stdout/stderr
 *     { "type": "exit",   "code": 0 }    engine process ended
 *     { "type": "error",  "message":"…"} spawn/other failure
 *
 * Security: by default the bridge binds to 127.0.0.1 only, so it is reachable
 * solely from this machine. It will launch whatever executable path the browser
 * sends; keep it that way and do not expose the port to your network. An
 * optional allow-list (ENGINES_DIR) restricts launchable paths to one folder.
 *
 * Usage:
 *   node bridge/server.mjs                 # listens on ws://127.0.0.1:8181
 *   PORT=9000 node bridge/server.mjs       # custom port
 *   ENGINES_DIR=/path/to/engines node bridge/server.mjs   # restrict to a dir
 *
 * Requires the `ws` package: `npm install` inside the bridge/ folder, or run
 * `npm run bridge` from the project root after installing dev deps.
 */
import { WebSocketServer } from 'ws'
import { spawn } from 'node:child_process'
import { createInterface } from 'node:readline'
import path from 'node:path'
import fs from 'node:fs'

const PORT = Number(process.env.PORT) || 8181
const HOST = process.env.HOST || '127.0.0.1'
// Optional: restrict launchable engine binaries to this directory (recommended
// if you ever change HOST away from localhost).
const ENGINES_DIR = process.env.ENGINES_DIR
  ? path.resolve(process.env.ENGINES_DIR)
  : null

const wss = new WebSocketServer({ host: HOST, port: PORT })

console.log(`[bridge] listening on ws://${HOST}:${PORT}`)
if (ENGINES_DIR) {
  console.log(`[bridge] engine paths restricted to: ${ENGINES_DIR}`)
}

/**
 * Validate that a requested engine path is allowed and executable.
 * Returns the resolved absolute path, or throws with a reason.
 */
const resolveEnginePath = enginePath => {
  if (typeof enginePath !== 'string' || !enginePath.trim()) {
    throw new Error('Engine path is empty.')
  }
  const resolved = path.resolve(enginePath)

  if (ENGINES_DIR) {
    // Ensure resolved path stays inside the allow-listed directory.
    const rel = path.relative(ENGINES_DIR, resolved)
    if (rel.startsWith('..') || path.isAbsolute(rel)) {
      throw new Error(`Engine path is outside ENGINES_DIR: ${resolved}`)
    }
  }

  if (!fs.existsSync(resolved)) {
    throw new Error(`Engine not found: ${resolved}`)
  }
  const stat = fs.statSync(resolved)
  if (!stat.isFile()) {
    throw new Error(`Engine path is not a file: ${resolved}`)
  }
  return resolved
}

wss.on('connection', (ws, req) => {
  const peer = req.socket.remoteAddress
  console.log(`[bridge] client connected: ${peer}`)

  /** @type {import('node:child_process').ChildProcess | null} */
  let child = null

  const send = obj => {
    if (ws.readyState === ws.OPEN) ws.send(JSON.stringify(obj))
  }

  const killChild = () => {
    if (child) {
      try {
        child.kill()
      } catch {
        /* ignore */
      }
      child = null
    }
  }

  const spawnEngine = (enginePath, args) => {
    killChild()

    let resolved
    try {
      resolved = resolveEnginePath(enginePath)
    } catch (e) {
      send({ type: 'error', message: e.message })
      return
    }

    try {
      // Run the engine with its own directory as cwd so it can locate its NNUE
      // network file, mirroring the desktop app's behavior.
      child = spawn(resolved, Array.isArray(args) ? args : [], {
        cwd: path.dirname(resolved),
        stdio: ['pipe', 'pipe', 'pipe'],
      })
    } catch (e) {
      send({ type: 'error', message: `Failed to spawn engine: ${e.message}` })
      return
    }

    // Relay stdout and stderr line by line.
    const rlOut = createInterface({ input: child.stdout })
    rlOut.on('line', line => send({ type: 'output', line }))
    const rlErr = createInterface({ input: child.stderr })
    rlErr.on('line', line => send({ type: 'output', line }))

    child.on('error', e =>
      send({ type: 'error', message: `Engine process error: ${e.message}` })
    )
    child.on('exit', code => {
      send({ type: 'exit', code: code ?? 0 })
      child = null
    })

    send({ type: 'spawned' })
    console.log(`[bridge] spawned engine: ${resolved}`)
  }

  ws.on('message', data => {
    let msg
    try {
      msg = JSON.parse(data.toString())
    } catch {
      send({ type: 'error', message: 'Invalid JSON message.' })
      return
    }

    switch (msg.type) {
      case 'spawn':
        spawnEngine(msg.path, msg.args)
        break
      case 'cmd':
        if (child && child.stdin.writable) {
          child.stdin.write(`${msg.command}\n`)
        }
        break
      case 'kill':
        killChild()
        break
      default:
        send({ type: 'error', message: `Unknown message type: ${msg.type}` })
    }
  })

  ws.on('close', () => {
    console.log(`[bridge] client disconnected: ${peer}`)
    killChild()
  })
})
