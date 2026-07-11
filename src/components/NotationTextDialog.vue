<template>
  <v-dialog
    v-model="dialogVisible"
    persistent
    max-width="800px"
    :fullscreen="$vuetify.display.smAndDown"
  >
    <v-card>
      <DialogHeader
        :title="$t('notationTextDialog.title')"
        :subtitle="$t('notationTextDialog.subtitle')"
        icon="mdi-clipboard-text"
        @close="closeDialog"
      />

      <v-card-text class="notation-body">
        <v-textarea
          v-model="notationText"
          :label="$t('notationTextDialog.placeholder')"
          variant="outlined"
          rows="12"
          auto-grow
          clearable
          hide-details
          spellcheck="false"
          class="notation-editor"
        />
        <p class="notation-hint">{{ $t('notationTextDialog.hint') }}</p>
      </v-card-text>

      <v-card-actions class="notation-actions">
        <v-btn
          variant="text"
          prepend-icon="mdi-content-copy"
          @click="copyToClipboard"
          :loading="isCopying"
        >
          {{ $t('notationTextDialog.copy') }}
        </v-btn>
        <v-spacer />
        <v-btn variant="text" @click="closeDialog">
          {{ $t('common.close') }}
        </v-btn>
        <v-btn color="primary" variant="flat" @click="apply">
          {{ $t('notationTextDialog.apply') }}
        </v-btn>
      </v-card-actions>
    </v-card>

    <v-snackbar v-model="showCopyStatus" :timeout="2000" :color="copyStatusColor">
      {{ copyStatusText }}
    </v-snackbar>
  </v-dialog>
</template>

<script setup lang="ts">
  import { ref, computed, inject, watch } from 'vue'
  import { useI18n } from 'vue-i18n'
  import DialogHeader from './DialogHeader.vue'

  const { t } = useI18n()

  interface Props {
    modelValue: boolean
  }

  interface Emits {
    (e: 'update:modelValue', value: boolean): void
    (e: 'apply', text: string): void
  }

  const props = defineProps<Props>()
  const emit = defineEmits<Emits>()

  // Inject game state to get current notation
  const gameState: any = inject('game-state')

  const notationText = ref('')
  const isCopying = ref(false)
  const showCopyStatus = ref(false)
  const copyStatusText = ref('')
  const copyStatusColor = ref<'success' | 'error'>('success')

  const dialogVisible = computed({
    get: () => props.modelValue,
    set: v => emit('update:modelValue', v),
  })

  // Populate on open with current notation JSON string
  watch(
    () => dialogVisible.value,
    opened => {
      if (opened && gameState?.generateGameNotation) {
        try {
          const notationObj = gameState.generateGameNotation()
          notationText.value = JSON.stringify(notationObj, null, 2)
        } catch (e) {
          // Fallback to empty text on error
          notationText.value = ''
        }
      }
    }
  )

  const closeDialog = () => {
    dialogVisible.value = false
  }

  const copyToClipboard = async () => {
    try {
      isCopying.value = true
      await navigator.clipboard.writeText(notationText.value || '')
      copyStatusColor.value = 'success'
      copyStatusText.value = t('notationTextDialog.copied')
    } catch (e) {
      console.error('Failed to copy notation JSON:', e)
      copyStatusColor.value = 'error'
      copyStatusText.value = t('notationTextDialog.copyFailed')
    } finally {
      isCopying.value = false
      showCopyStatus.value = true
    }
  }

  const apply = () => {
    emit('apply', notationText.value || '')
  }
</script>

<style lang="scss" scoped>
  .notation-body {
    padding: 16px 24px 8px;
  }

  // The payload is JSON — render it in the mono grid like the rest of the data.
  .notation-editor :deep(textarea) {
    font-family: var(--jb-mono, monospace);
    font-size: 13px;
    line-height: 1.5;
  }

  .notation-hint {
    margin: 10px 0 0;
    font-size: 12px;
    line-height: 1.4;
    color: rgba(var(--v-theme-on-surface), 0.55);
  }

  .notation-actions {
    padding: 12px 20px;
    border-top: 1px solid var(--jb-line, rgba(var(--v-border-color), 0.16));
  }
</style>
