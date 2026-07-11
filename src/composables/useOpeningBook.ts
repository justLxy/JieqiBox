// Opening book composable for managing opening book functionality
import { ref, computed, reactive, watch } from 'vue'
import { isTauri } from '@/utils/runtime'
import {
  webAddEntry,
  webDeleteEntry,
  webQueryMoves,
  webImportEntries,
  webExportAll,
  webGetStats,
  webClearAll,
} from './openingBook/webOpeningBook'
import type {
  MoveData,
  OpeningBookEntry,
  OpeningBookStats,
  OpeningBookImportResult,
  JieqiOpeningBookConfig,
} from '@/types/openingBook'
import { useInterfaceSettings } from './useInterfaceSettings'

export interface OpeningBookEntryInput {
  fen: string
  uciMove: string
  priority?: number
  wins?: number
  draws?: number
  losses?: number
  allowed?: boolean
  comment?: string
}

// Lazy Tauri invoke so the web bundle never loads the native core.
const tauriInvoke = async <T>(
  cmd: string,
  args?: Record<string, unknown>
): Promise<T> => {
  const { invoke } = await import('@tauri-apps/api/core')
  return invoke<T>(cmd, args)
}

export function useOpeningBook() {
  const isInitialized = ref(true) // SQLite database is always available
  const isLoading = ref(false)
  const error = ref<string | null>(null)

  // Get persistent settings
  const {
    showBookMoves,
    openingBookEnableInGame,
    openingBookPreferHighPriority,
  } = useInterfaceSettings()

  // Configuration - now uses persistent settings
  const config = reactive<JieqiOpeningBookConfig>({
    dbPath: 'jieqi_openings.jb',
    autoLoad: true,
    enableInGame: openingBookEnableInGame.value,
    showBookMoves: showBookMoves.value,
    preferHighPriority: openingBookPreferHighPriority.value,
  })

  // Sync config with persistent settings
  watch(
    [openingBookEnableInGame, showBookMoves, openingBookPreferHighPriority],
    ([newEnableInGame, newShowBookMoves, newPreferHighPriority]) => {
      config.enableInGame = newEnableInGame
      config.showBookMoves = newShowBookMoves
      config.preferHighPriority = newPreferHighPriority
    }
  )

  // Statistics
  const stats = ref<OpeningBookStats>({
    totalPositions: 0,
    totalMoves: 0,
    allowedMoves: 0,
    disallowedMoves: 0,
  })

  // Current position book moves
  const currentBookMoves = ref<MoveData[]>([])

  // Initialize the opening book (just update stats since SQLite is always available)
  const initialize = async (): Promise<void> => {
    try {
      isLoading.value = true
      error.value = null
      await updateStats()
    } catch (err) {
      error.value =
        err instanceof Error ? err.message : 'Failed to initialize opening book'
      console.error('Opening book initialization error:', err)
    } finally {
      isLoading.value = false
    }
  }

  // Add a new entry to the opening book
  const addEntry = async (
    fen: string,
    uciMove: string,
    priority: number = 100,
    wins: number = 0,
    draws: number = 0,
    losses: number = 0,
    allowed: boolean = true,
    comment: string = ''
  ): Promise<boolean> => {
    try {
      const success = isTauri()
        ? await tauriInvoke<boolean>('opening_book_add_entry', {
            request: {
              fen,
              uci_move: uciMove,
              priority,
              wins,
              draws,
              losses,
              allowed,
              comment,
            },
          })
        : webAddEntry(
            fen,
            uciMove,
            priority,
            wins,
            draws,
            losses,
            allowed,
            comment
          )
      if (success) {
        await updateStats()
      }
      return success
    } catch (err) {
      error.value = err instanceof Error ? err.message : 'Failed to add entry'
      console.error('Opening book add entry error:', err)
      return false
    }
  }

  // Add several entries with one native transaction and one statistics refresh.
  const addEntries = async (
    entries: OpeningBookEntryInput[]
  ): Promise<boolean> => {
    if (entries.length === 0) return true

    try {
      const success = isTauri()
        ? await tauriInvoke<boolean>('opening_book_add_entries', {
            requests: entries.map(entry => ({
              fen: entry.fen,
              uci_move: entry.uciMove,
              priority: entry.priority ?? 100,
              wins: entry.wins ?? 0,
              draws: entry.draws ?? 0,
              losses: entry.losses ?? 0,
              allowed: entry.allowed ?? true,
              comment: entry.comment ?? '',
            })),
          })
        : entries.every(entry =>
            webAddEntry(
              entry.fen,
              entry.uciMove,
              entry.priority ?? 100,
              entry.wins ?? 0,
              entry.draws ?? 0,
              entry.losses ?? 0,
              entry.allowed ?? true,
              entry.comment ?? ''
            )
          )
      if (success) {
        await updateStats()
      }
      return success
    } catch (err) {
      error.value =
        err instanceof Error
          ? err.message
          : 'Failed to add opening book entries'
      console.error('Opening book add entries error:', err)
      return false
    }
  }

  // Delete an entry from the opening book
  const deleteEntry = async (
    fen: string,
    uciMove: string
  ): Promise<boolean> => {
    try {
      const success = isTauri()
        ? await tauriInvoke<boolean>('opening_book_delete_entry', {
            fen,
            uciMove,
          })
        : webDeleteEntry(fen, uciMove)
      if (success) {
        await updateStats()
      }
      return success
    } catch (err) {
      error.value =
        err instanceof Error ? err.message : 'Failed to delete entry'
      console.error('Opening book delete entry error:', err)
      return false
    }
  }

  // Query moves for a given position (independent from enableInGame; UI controls visibility)
  const queryMoves = async (fen: string): Promise<MoveData[]> => {
    try {
      const moves = isTauri()
        ? await tauriInvoke<MoveData[]>('opening_book_query_moves', { fen })
        : webQueryMoves(fen)
      currentBookMoves.value = moves
      return moves
    } catch (err) {
      error.value = err instanceof Error ? err.message : 'Failed to query moves'
      console.error('Opening book query moves error:', err)
      return []
    }
  }

  // Get the best move for a position (used for auto-play when enableInGame is true)
  const getBestMove = async (fen: string): Promise<string | null> => {
    // Check if opening book is enabled in game
    if (!config.enableInGame) {
      return null
    }

    const moves = await queryMoves(fen)
    if (moves.length === 0) return null

    // Filter allowed moves only
    const allowedMoves = moves.filter(move => move.allowed)
    if (allowedMoves.length === 0) return null

    if (config.preferHighPriority) {
      const maxPriority = Math.max(...allowedMoves.map(m => m.priority))
      const topMoves = allowedMoves.filter(m => m.priority === maxPriority)
      // Randomize among same-priority moves
      const randomIndex = Math.floor(Math.random() * topMoves.length)
      return topMoves[randomIndex].uci_move
    }

    // Random selection from allowed moves
    const randomIndex = Math.floor(Math.random() * allowedMoves.length)
    return allowedMoves[randomIndex].uci_move
  }

  // Import opening book data
  const importData = async (
    data: OpeningBookEntry[]
  ): Promise<OpeningBookImportResult> => {
    try {
      const [imported, errors] = isTauri()
        ? await tauriInvoke<[number, string[]]>('opening_book_import_entries', {
            jsonData: JSON.stringify(data),
          })
        : webImportEntries(data)

      await updateStats()

      return {
        success: errors.length === 0,
        imported,
        errors,
        duplicates: 0, // SQLite handles duplicates automatically
      }
    } catch (err) {
      return {
        success: false,
        imported: 0,
        errors: [err instanceof Error ? err.message : 'Import failed'],
        duplicates: 0,
      }
    }
  }

  // Export opening book data
  const exportData = async (): Promise<OpeningBookEntry[]> => {
    try {
      if (!isTauri()) return webExportAll()
      const jsonData = await tauriInvoke<string>('opening_book_export_all')
      return JSON.parse(jsonData)
    } catch (err) {
      error.value = err instanceof Error ? err.message : 'Failed to export data'
      console.error('Opening book export error:', err)
      return []
    }
  }

  // Update statistics
  const updateStats = async (): Promise<void> => {
    try {
      const raw = isTauri()
        ? await tauriInvoke<any>('opening_book_get_stats')
        : webGetStats()
      // Map snake_case from backend to camelCase used in frontend
      const mapped: OpeningBookStats = {
        totalPositions:
          (raw && (raw.totalPositions ?? raw.total_positions)) ?? 0,
        totalMoves: (raw && (raw.totalMoves ?? raw.total_moves)) ?? 0,
        allowedMoves: (raw && (raw.allowedMoves ?? raw.allowed_moves)) ?? 0,
        disallowedMoves:
          (raw && (raw.disallowedMoves ?? raw.disallowed_moves)) ?? 0,
      }
      stats.value = mapped
    } catch (err) {
      error.value =
        err instanceof Error ? err.message : 'Failed to update stats'
      console.error('Opening book stats error:', err)
    }
  }

  // Clear all data
  const clearAll = async (): Promise<boolean> => {
    try {
      if (isTauri()) await tauriInvoke<void>('opening_book_clear_all')
      else webClearAll()
      await updateStats()
      currentBookMoves.value = []
      return true
    } catch (err) {
      error.value = err instanceof Error ? err.message : 'Failed to clear data'
      console.error('Opening book clear error:', err)
      return false
    }
  }

  // Computed properties
  const hasBookMoves = computed(() => currentBookMoves.value.length > 0)
  const allowedBookMoves = computed(() =>
    currentBookMoves.value.filter(move => move.allowed)
  )
  const isBookLoaded = computed(() => stats.value.totalMoves > 0)

  // Cleanup (no-op for SQLite backend)
  const cleanup = (): void => {
    currentBookMoves.value = []
    error.value = null
  }

  return {
    // State
    isInitialized,
    isLoading,
    error,
    config,
    stats,
    currentBookMoves,

    // Actions
    initialize,
    addEntry,
    addEntries,
    deleteEntry,
    queryMoves,
    getBestMove,
    importData,
    exportData,
    updateStats,
    clearAll,
    cleanup,

    // Computed
    hasBookMoves,
    allowedBookMoves,
    isBookLoaded,
  }
}
