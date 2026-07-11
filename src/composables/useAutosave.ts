import { ref, watch } from 'vue'
import { useInterfaceSettings } from './useInterfaceSettings'
import { loadAutosaveString, saveAutosaveString } from '../utils/storage'

const AUTOSAVE_INTERVAL_MS = 5000

// Autosave functionality composable
export function useAutosave() {
  const { autosave } = useInterfaceSettings()

  const isAutosaveEnabled = ref(false)
  const lastAutosaveTime = ref(0)

  let autosaveTimer: ReturnType<typeof setInterval> | null = null
  let currentGameState: any = null
  let isInitialized = false
  let pendingGameState: any = null
  let autosaveWritePromise: Promise<void> | null = null
  let lastSavedContent: string | null = null

  const loadAutosaveOnStartup = async (gameState: any) => {
    try {
      const autosaveContent = await loadAutosaveString()
      if (!autosaveContent?.trim() || !gameState) return

      const gameData = JSON.parse(autosaveContent)
      if (!gameData?.metadata?.currentFen) return

      // Reuse the normal notation loader so initial FEN, flip mode, comments,
      // and any future notation fields restore consistently with manual open.
      if (typeof gameState.loadGameNotationFromText === 'function') {
        const loaded = await gameState.loadGameNotationFromText(autosaveContent)
        if (loaded) {
          lastSavedContent = JSON.stringify(gameData)
          return
        }
      }

      // Compatibility fallback for integrations exposing only the older API.
      gameState.loadFen(gameData.metadata.currentFen, false)
      if (Array.isArray(gameData.moves)) {
        gameState.history.value = [...gameData.moves]
        gameState.currentMoveIndex.value = gameData.moves.length
      }
      lastSavedContent = JSON.stringify(gameData)
    } catch (error) {
      console.error('Failed to load autosave on startup:', error)
    }
  }

  const saveAutosave = async (gameState: any) => {
    if (!isAutosaveEnabled.value || !gameState) return

    // A slow disk write must never let interval ticks queue concurrent JSON
    // serializations. Keep only the latest requested game state.
    pendingGameState = gameState

    if (!autosaveWritePromise) {
      autosaveWritePromise = (async () => {
        while (pendingGameState && isAutosaveEnabled.value) {
          const stateToSave = pendingGameState
          pendingGameState = null

          try {
            const content = JSON.stringify(stateToSave.generateGameNotation())
            if (content === lastSavedContent) continue

            await saveAutosaveString(content)
            lastSavedContent = content
            lastAutosaveTime.value = Date.now()
          } catch (error) {
            console.error('Failed to save autosave:', error)
          }
        }
        pendingGameState = null
      })().finally(() => {
        autosaveWritePromise = null
      })
    }

    await autosaveWritePromise
  }

  const stopAutosaveTimer = () => {
    if (autosaveTimer) {
      clearInterval(autosaveTimer)
      autosaveTimer = null
    }
    pendingGameState = null
  }

  const startAutosaveTimer = (gameState: any) => {
    stopAutosaveTimer()
    currentGameState = gameState

    if (!isAutosaveEnabled.value || !currentGameState) return

    autosaveTimer = setInterval(() => {
      void saveAutosave(currentGameState)
    }, AUTOSAVE_INTERVAL_MS)
  }

  watch(autosave, enabled => {
    isAutosaveEnabled.value = enabled

    if (!isInitialized) return
    if (enabled && currentGameState) {
      startAutosaveTimer(currentGameState)
    } else {
      stopAutosaveTimer()
    }
  })

  const initializeAutosave = async (gameState: any) => {
    if (isInitialized) return

    try {
      currentGameState = gameState
      isAutosaveEnabled.value = autosave.value

      if (isAutosaveEnabled.value) {
        await loadAutosaveOnStartup(gameState)
      }

      isInitialized = true
      if (isAutosaveEnabled.value) {
        startAutosaveTimer(gameState)
      }
    } catch (error) {
      console.error('Failed to initialize autosave:', error)
    }
  }

  return {
    isAutosaveEnabled,
    lastAutosaveTime,
    saveAutosave,
    loadAutosaveOnStartup,
    startAutosaveTimer,
    stopAutosaveTimer,
    initializeAutosave,
  }
}
