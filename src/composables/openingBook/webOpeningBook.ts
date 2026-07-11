/**
 * Web opening-book provider.
 *
 * The native build stores the opening book in a SQLite database managed by the
 * Rust backend. In the browser there is no SQLite process, so this module
 * provides an equivalent key/value store backed by `localStorage`, keyed by a
 * normalized FEN. The data shape mirrors what the Tauri commands return so the
 * `useOpeningBook` composable can call either backend transparently.
 *
 * The store is a JSON object: { [fenKey: string]: MoveData[] }. This is fine
 * for the typical hand-curated opening book; very large imported books would be
 * better served by IndexedDB, which can replace this later without touching the
 * composable.
 */
import type { MoveData, OpeningBookEntry } from '@/types/openingBook'

const STORE_KEY = 'jieqibox.openingbook'

type BookStore = Record<string, MoveData[]>

const readStore = (): BookStore => {
  try {
    const raw = localStorage.getItem(STORE_KEY)
    return raw ? (JSON.parse(raw) as BookStore) : {}
  } catch {
    return {}
  }
}

const writeStore = (store: BookStore): void => {
  localStorage.setItem(STORE_KEY, JSON.stringify(store))
}

/**
 * The native backend normalizes FENs (mirror/color) before hashing. For the web
 * store we key on the position portion of the FEN (board + side to move),
 * ignoring move counters, which is a reasonable approximation for a manual book.
 */
const fenKey = (fen: string): string => {
  const parts = fen.trim().split(/\s+/)
  // board, side, (optional) extra fields — drop halfmove/fullmove counters.
  return parts.slice(0, 2).join(' ')
}

export const webAddEntry = (
  fen: string,
  uciMove: string,
  priority: number,
  wins: number,
  draws: number,
  losses: number,
  allowed: boolean,
  comment: string
): boolean => {
  const store = readStore()
  const key = fenKey(fen)
  const moves = store[key] ?? []
  const existing = moves.find(m => m.uci_move === uciMove)
  if (existing) {
    existing.priority = priority
    existing.wins = wins
    existing.draws = draws
    existing.losses = losses
    existing.allowed = allowed
    existing.comment = comment
  } else {
    moves.push({
      uci_move: uciMove,
      priority,
      wins,
      draws,
      losses,
      allowed,
      comment,
    })
  }
  store[key] = moves
  writeStore(store)
  return true
}

export const webDeleteEntry = (fen: string, uciMove: string): boolean => {
  const store = readStore()
  const key = fenKey(fen)
  const moves = store[key]
  if (!moves) return false
  const next = moves.filter(m => m.uci_move !== uciMove)
  if (next.length > 0) store[key] = next
  else delete store[key]
  writeStore(store)
  return true
}

export const webQueryMoves = (fen: string): MoveData[] => {
  const store = readStore()
  return store[fenKey(fen)] ?? []
}

export const webImportEntries = (
  entries: OpeningBookEntry[]
): [number, string[]] => {
  const store = readStore()
  let imported = 0
  const errors: string[] = []
  for (const entry of entries) {
    try {
      const key = fenKey(entry.fen)
      store[key] = entry.moves
      imported += entry.moves.length
    } catch (e: any) {
      errors.push(e?.message || String(e))
    }
  }
  writeStore(store)
  return [imported, errors]
}

export const webExportAll = (): OpeningBookEntry[] => {
  const store = readStore()
  return Object.entries(store).map(([key, moves]) => ({
    key,
    fen: key,
    moves,
  }))
}

export const webGetStats = () => {
  const store = readStore()
  let totalMoves = 0
  let allowedMoves = 0
  for (const moves of Object.values(store)) {
    totalMoves += moves.length
    allowedMoves += moves.filter(m => m.allowed).length
  }
  return {
    totalPositions: Object.keys(store).length,
    totalMoves,
    allowedMoves,
    disallowedMoves: totalMoves - allowedMoves,
  }
}

export const webClearAll = (): void => {
  localStorage.removeItem(STORE_KEY)
}
