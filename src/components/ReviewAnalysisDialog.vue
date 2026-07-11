<template>
  <v-dialog v-model="isVisible" width="460" persistent>
    <v-card>
      <DialogHeader
        :title="$t('reviewDialog.title')"
        :subtitle="$t('reviewDialog.subtitle')"
        icon="mdi-clipboard-pulse"
        @close="handleCancel"
      />
      <v-card-text class="review-body">
        <p class="review-intro">{{ $t('reviewDialog.description') }}</p>

        <label class="field-label">{{ $t('reviewDialog.movetime') }}</label>
        <v-text-field
          v-model.number="reviewMovetime"
          type="number"
          density="compact"
          variant="outlined"
          min="100"
          hide-details
          suffix="ms"
          class="mono-input review-input"
          :disabled="isReviewing"
        />
        <p class="field-hint">{{ $t('reviewDialog.movetimeHint') }}</p>

        <div v-if="isReviewing" class="review-progress">
          <div class="review-progress__row">
            <span>{{ $t('reviewDialog.analyzing') }}</span>
            <span class="review-progress__count"
              >{{ reviewProgress }} / {{ reviewTotal }}</span
            >
          </div>
          <v-progress-linear
            :model-value="progressPercent"
            color="primary"
            height="6"
            rounded
          />
        </div>

        <v-alert
          v-if="showNoEngine"
          type="warning"
          variant="tonal"
          density="compact"
          class="mt-3"
          >{{ $t('analysis.noEngineLoaded') }}</v-alert
        >
      </v-card-text>
      <v-card-actions class="review-actions">
        <v-spacer />
        <v-btn variant="text" @click="handleCancel" :disabled="isStopping">{{
          isReviewing ? $t('common.cancel') : $t('common.close')
        }}</v-btn>
        <v-btn
          color="primary"
          variant="flat"
          @click="startReview"
          :loading="isReviewing"
          :disabled="isReviewing"
          >{{ $t('common.execute') }}</v-btn
        >
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<script setup lang="ts">
  import { computed, inject, nextTick, ref, watch } from 'vue'
  import DialogHeader from './DialogHeader.vue'

  const props = defineProps<{ modelValue: boolean }>()
  const emit = defineEmits<{ (e: 'update:modelValue', v: boolean): void }>()

  const gameState: any = inject('game-state')
  const engineState: any = inject('engine-state')

  const isVisible = computed({
    get: () => props.modelValue,
    set: v => emit('update:modelValue', v),
  })

  const reviewMovetime = ref<number>(1000)
  const isReviewing = ref(false)
  const isStopping = ref(false)
  const reviewProgress = ref(0)
  const reviewTotal = ref(0)
  const cancelRequested = ref(false)
  const showNoEngine = ref(false)

  const progressPercent = computed(() =>
    reviewTotal.value > 0
      ? Math.round((reviewProgress.value / reviewTotal.value) * 100)
      : 0
  )

  // Clear the "no engine" warning whenever the dialog is reopened.
  watch(isVisible, v => {
    if (v) showNoEngine.value = false
  })

  const MATE_SCORE_BASE = 30000

  const waitForEngineStop = (): Promise<void> => {
    return new Promise(resolve => {
      if (!engineState.isThinking?.value) {
        resolve()
        return
      }
      const unwatch = watch(
        () => engineState.isThinking?.value,
        thinking => {
          if (!thinking) {
            unwatch()
            resolve()
          }
        }
      )
    })
  }

  const extractScoreFromOutput = (
    startIndex = 0
  ): { score: number | null; depth?: number; nodes?: number } => {
    try {
      const out: any[] = engineState.engineOutput?.value || []

      // If the buffer was cleared during analysis, clamp lowerBound to 0
      const lowerBound =
        startIndex > 0 && startIndex <= out.length - 1 ? startIndex : 0

      let lastLine = ''
      for (let i = out.length - 1; i >= lowerBound; i--) {
        const line = out[i]
        const text = line?.text || ''
        if (line?.kind === 'recv' && text.includes('score')) {
          lastLine = text
          break
        }
      }

      if (!lastLine) {
        return { score: null }
      }

      const m = lastLine.match(/score\s+(?:(cp|mate)\s+)?(-?\d+)/)
      if (!m) {
        return { score: null }
      }

      const type = m[1] || 'cp'
      const val = parseInt(m[2])
      let cp: number
      if (type === 'mate') {
        const ply = Math.abs(val)
        // In UCI, mate 0 indicates side-to-move is checkmated (losing), treat as negative
        const sign = val === 0 ? -1 : val > 0 ? 1 : -1
        cp = sign * (MATE_SCORE_BASE - ply)
      } else {
        cp = val
      }

      const depthMatch = lastLine.match(/depth\s+(\d+)/)
      const nodesMatch = lastLine.match(/nodes\s+(\d+)/)

      return {
        score: cp,
        depth: depthMatch ? parseInt(depthMatch[1]) : undefined,
        nodes: nodesMatch ? parseInt(nodesMatch[1]) : undefined,
      }
    } catch (e) {
      console.error('Review: failed to extract score from engine output:', e)
      return { score: null }
    }
  }

  const analyzeOnce = async (
    fen: string,
    movetimeMs: number
  ): Promise<{
    score: number | null
    depth?: number
    nodes?: number
    timeUsed?: number
  }> => {
    const startLen = (engineState.engineOutput?.value || []).length
    const settings = {
      movetime: Math.max(100, Math.floor(movetimeMs)),
      maxThinkTime: 0,
      maxDepth: 0,
      maxNodes: 0,
      analysisMode: 'movetime',
    }

    if (engineState.stopAnalysis) {
      engineState.stopAnalysis({ playBestMoveOnStop: false })
      await waitForEngineStop()
    }
    const startTs = Date.now()
    engineState.startAnalysis(settings, [], fen, [])
    await waitForEngineStop()
    const timeUsed = Date.now() - startTs

    let { score, depth, nodes } = extractScoreFromOutput(startLen)
    // Fallback if nothing found (e.g., buffer cleared during analysis)
    if (score === null) {
      const retry = extractScoreFromOutput(0)
      score = retry.score
      depth = retry.depth
      nodes = retry.nodes
    }

    return { score, depth, nodes, timeUsed }
  }

  const pickAnnotation = (
    delta: number,
    sBefore: number | null,
    sAfter: number | null
  ): '!!' | '!' | '!?' | '?!' | '?' | '??' | undefined => {
    // Check if this is a mate position (before or after move)
    const isMateBefore =
      sBefore !== null && Math.abs(sBefore) > MATE_SCORE_BASE - 100
    const isMateAfter =
      sAfter !== null && Math.abs(sAfter) > MATE_SCORE_BASE - 100

    // Handle mate situations
    if (isMateBefore || isMateAfter) {
      // Case 1: Had a mate but threw it away (mate miss - 漏杀)
      if (
        isMateBefore &&
        sBefore! > MATE_SCORE_BASE - 100 &&
        (!isMateAfter || sAfter! < MATE_SCORE_BASE - 100)
      ) {
        return '??' // Threw away a winning mate - this is a blunder
      }

      // Case 2: Was losing but opponent gave mate back
      if (
        isMateBefore &&
        sBefore! < -(MATE_SCORE_BASE - 100) &&
        isMateAfter &&
        sAfter! > MATE_SCORE_BASE - 100
      ) {
        return '!!' // Opponent blundered mate back - excellent
      }

      // Case 3: Had mate, still have mate but took longer route
      if (
        isMateBefore &&
        sBefore! > MATE_SCORE_BASE - 100 &&
        isMateAfter &&
        sAfter! > MATE_SCORE_BASE - 100
      ) {
        const plyBefore = MATE_SCORE_BASE - sBefore!
        const plyAfter = MATE_SCORE_BASE - sAfter!
        if (plyAfter > plyBefore + 2) {
          // Took significantly longer mate
          return '?' // Inaccurate but not a blunder
        }
        return undefined // Still mate, reasonable path
      }

      // Case 4: Other mate situations - significant score changes
      if (delta <= -500) return '??'
      if (delta >= 500) return '!!'
      return undefined
    }

    // Normal (non-mate) position evaluation
    if (delta <= -300) return '??'
    if (delta <= -150) return '?'
    if (delta <= -60) return '?!'
    if (delta >= 300) return '!!'
    if (delta >= 150) return '!'
    if (delta >= 60) return '!?'
    return undefined
  }

  const startReview = async () => {
    if (!engineState?.isEngineLoaded?.value) {
      showNoEngine.value = true
      return
    }
    showNoEngine.value = false
    try {
      isReviewing.value = true
      isStopping.value = false
      cancelRequested.value = false
      reviewProgress.value = 0
      const history: any[] = gameState.history?.value || []
      const total = history.filter(e => e.type === 'move').length
      reviewTotal.value = total

      for (let idx = history.length - 1; idx >= 0; idx--) {
        const entry = history[idx]
        if (!entry || entry.type !== 'move') continue

        // Replay UI to the position before this move for correct analysis context
        gameState.replayToMove?.(idx)
        await nextTick()

        const fenBefore =
          idx === 0 ? gameState.initialFen?.value : history[idx - 1].fen
        const fenAfter = entry.fen

        // Analyze position before the move (mover to play)
        const beforeRes = await analyzeOnce(fenBefore, reviewMovetime.value)
        if (cancelRequested.value) break

        // Analyze position after the move (opponent to play)
        const afterRes = await analyzeOnce(fenAfter, reviewMovetime.value)
        if (cancelRequested.value) break

        const sBefore = beforeRes.score ?? 0
        const sAfter = afterRes.score ?? 0
        // Convert sAfter to mover's perspective (opponent's eval from our perspective is negated)
        const sAfterFromMoverPerspective = -sAfter
        // Change for mover: eval_after - eval_before (both from mover's perspective)
        const delta = sAfterFromMoverPerspective - sBefore

        // Record the score for the position before the move (what we analyzed)
        const updated = {
          ...entry,
          engineScore: sBefore, // Score of the position before the move
          engineTime: reviewMovetime.value,
          engineDepth: beforeRes.depth,
          engineNodes: beforeRes.nodes,
          engineRequestedMovetime: reviewMovetime.value,
        }
        gameState.history.value[idx] = updated

        const ann = pickAnnotation(
          delta,
          beforeRes.score,
          sAfterFromMoverPerspective
        )
        gameState.updateMoveAnnotation?.(idx, ann)

        reviewProgress.value++
      }
    } catch (e) {
      console.error('Review analysis failed:', e)
    } finally {
      isStopping.value = true
      if (engineState.stopAnalysis) {
        engineState.stopAnalysis({ playBestMoveOnStop: false })
        await waitForEngineStop()
      }
      isStopping.value = false
      isReviewing.value = false
      if (!cancelRequested.value) isVisible.value = false
    }
  }

  const handleCancel = async () => {
    if (isReviewing.value) {
      cancelRequested.value = true
      isStopping.value = true
      if (engineState.stopAnalysis) {
        engineState.stopAnalysis({ playBestMoveOnStop: false })
        await waitForEngineStop()
      }
      isStopping.value = false
      isReviewing.value = false
    }
    isVisible.value = false
  }
</script>

<style lang="scss" scoped>
  .review-body {
    padding: 16px 24px 8px;
  }

  .review-intro {
    margin: 0 0 16px;
    font-size: 13px;
    line-height: 1.5;
    color: rgba(var(--v-theme-on-surface), 0.7);
  }

  .field-label {
    display: block;
    font-weight: 600;
    font-size: 14px;
    color: rgb(var(--v-theme-on-surface));
    margin-bottom: 8px;
  }

  .review-input {
    max-width: 200px;
  }

  .mono-input :deep(input) {
    font-family: var(--jb-mono, monospace);
    font-variant-numeric: tabular-nums;
  }

  .field-hint {
    margin: 8px 0 0;
    font-size: 12px;
    line-height: 1.4;
    color: rgba(var(--v-theme-on-surface), 0.55);
  }

  .review-progress {
    margin-top: 20px;
  }

  .review-progress__row {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin-bottom: 6px;
    font-size: 13px;
    color: rgb(var(--v-theme-on-surface));
  }

  .review-progress__count {
    font-family: var(--jb-mono, monospace);
    font-variant-numeric: tabular-nums;
    color: rgb(var(--v-theme-accent));
    font-weight: 600;
  }

  .review-actions {
    padding: 12px 20px;
    border-top: 1px solid var(--jb-line, rgba(var(--v-border-color), 0.16));
  }
</style>
