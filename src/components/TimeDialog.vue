<template>
  <v-dialog v-model="isVisible" :max-width="dialogMaxWidth" persistent scrollable>
    <v-card>
      <DialogHeader
        :title="$t('timeDialog.title')"
        :subtitle="$t('timeDialog.subtitle')"
        icon="mdi-timer-outline"
        @close="closeDialog"
      />

      <v-card-text class="settings-container">
        <!-- Analysis mode picker comes first: it drives which input shows. -->
        <div class="mode-picker">
          <label class="field-label">{{ $t('timeDialog.analysisMode') }}</label>
          <v-select
            v-model="analysisMode"
            :items="analysisModes"
            variant="outlined"
            density="compact"
            hide-details
            @update:model-value="updateSettings"
          ></v-select>
          <p class="mode-description">{{ currentModeDescription }}</p>
        </div>

        <v-divider class="my-4"></v-divider>

        <div class="setting-block" v-if="analysisMode === 'advanced'">
          <label class="field-label">{{ $t('timeDialog.advanced') }}</label>
          <div class="advanced-container">
            <div class="code-editor-container">
              <v-textarea
                v-model="advancedScript"
                rows="15"
                auto-grow
                variant="outlined"
                density="compact"
                hide-details
                class="code-editor"
                @update:model-value="updateSettings"
                :placeholder="advancedPlaceholder"
                spellcheck="false"
                wrap="off"
              />
              <div class="editor-overlay" v-if="advancedScript">
                <pre
                  class="syntax-highlight"
                  v-html="sanitizedHighlightedCode"
                ></pre>
              </div>
            </div>
            <v-expansion-panels class="mt-3">
              <v-expansion-panel>
                <v-expansion-panel-title>
                  <v-icon class="mr-2">mdi-code-tags</v-icon>
                  {{ $t('timeDialog.advancedExamples.title') }}
                </v-expansion-panel-title>
                <v-expansion-panel-text>
                  <div class="examples-container">
                    <div class="example-section">
                      <h4>{{ $t('timeDialog.advancedExamples.basic') }}</h4>
                      <pre class="code-block">{{
                        $t('timeDialog.advancedExamples.basicCode')
                      }}</pre>
                    </div>
                    <div class="example-section">
                      <h4>
                        {{ $t('timeDialog.advancedExamples.conditional') }}
                      </h4>
                      <pre class="code-block">{{
                        $t('timeDialog.advancedExamples.conditionalCode')
                      }}</pre>
                    </div>
                    <div class="example-section">
                      <h4>
                        {{ $t('timeDialog.advancedExamples.scoreBased') }}
                      </h4>
                      <pre class="code-block">{{
                        $t('timeDialog.advancedExamples.scoreBasedCode')
                      }}</pre>
                    </div>
                    <div class="example-section">
                      <h4>{{ $t('timeDialog.advancedExamples.variables') }}</h4>
                      <pre class="code-block">{{
                        $t('timeDialog.advancedExamples.variablesDesc')
                      }}</pre>
                    </div>
                  </div>
                </v-expansion-panel-text>
              </v-expansion-panel>
            </v-expansion-panels>
          </div>
        </div>
        <div class="setting-block" v-if="analysisMode === 'movetime'">
          <label class="field-label">{{ $t('timeDialog.movetime') }}</label>
          <v-text-field
            v-model.number="movetime"
            type="number"
            variant="outlined"
            density="compact"
            :min="100"
            :max="10000"
            :step="100"
            hide-details
            class="setting-input mono-input"
            suffix="ms"
            @update:model-value="updateSettings"
          ></v-text-field>
          <p class="field-hint">
            {{ $t('timeDialog.rangeHint', { min: 100, max: 10000 }) }}
          </p>
        </div>

        <div class="setting-block" v-if="analysisMode === 'maxThinkTime'">
          <label class="field-label">{{
            $t('timeDialog.maxThinkTime')
          }}</label>
          <v-text-field
            v-model.number="maxThinkTime"
            type="number"
            variant="outlined"
            density="compact"
            :min="100"
            :max="60000"
            :step="100"
            hide-details
            class="setting-input mono-input"
            suffix="ms"
            @update:model-value="updateSettings"
          ></v-text-field>
          <p class="field-hint">
            {{ $t('timeDialog.rangeHint', { min: 100, max: 60000 }) }}
          </p>
        </div>

        <div class="setting-block" v-if="analysisMode === 'depth'">
          <label class="field-label">{{ $t('timeDialog.maxDepth') }}</label>
          <v-text-field
            v-model.number="maxDepth"
            type="number"
            variant="outlined"
            density="compact"
            :min="1"
            :max="100"
            :step="1"
            hide-details
            class="setting-input mono-input"
            @update:model-value="updateSettings"
          ></v-text-field>
          <p class="field-hint">
            {{ $t('timeDialog.rangeHint', { min: 1, max: 100 }) }}
          </p>
        </div>

        <div class="setting-block" v-if="analysisMode === 'nodes'">
          <label class="field-label">{{ $t('timeDialog.maxNodes') }}</label>
          <v-text-field
            v-model.number="maxNodes"
            type="number"
            variant="outlined"
            density="compact"
            :min="1000"
            :max="10000000"
            :step="1000"
            hide-details
            class="setting-input mono-input"
            @update:model-value="updateSettings"
          ></v-text-field>
          <p class="field-hint">
            {{ $t('timeDialog.rangeHint', { min: 1000, max: 10000000 }) }}
          </p>
        </div>
      </v-card-text>

      <v-card-actions class="dialog-actions">
        <v-btn variant="text" @click="resetToDefaults">{{
          $t('timeDialog.resetToDefaults')
        }}</v-btn>
        <v-btn variant="text" color="error" @click="clearSettings">{{
          $t('timeDialog.clearSettings')
        }}</v-btn>
        <v-spacer></v-spacer>
        <v-btn color="primary" variant="flat" @click="closeDialog">{{
          $t('common.confirm')
        }}</v-btn>
      </v-card-actions>
    </v-card>

    <!-- Clear-settings confirmation (replaces native confirm()) -->
    <v-dialog v-model="showClearConfirm" max-width="420px">
      <v-card>
        <v-card-title class="confirm-title">{{
          $t('timeDialog.clearSettings')
        }}</v-card-title>
        <v-card-text>{{ $t('timeDialog.confirmClearSettings') }}</v-card-text>
        <v-card-actions>
          <v-spacer></v-spacer>
          <v-btn variant="text" @click="showClearConfirm = false">{{
            $t('common.cancel')
          }}</v-btn>
          <v-btn color="error" variant="flat" @click="confirmClearSettings">{{
            $t('common.confirm')
          }}</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </v-dialog>
</template>

<script setup lang="ts">
  import { ref, computed, onMounted } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { useConfigManager } from '../composables/useConfigManager'
  import DialogHeader from './DialogHeader.vue'
  import hljs from 'highlight.js/lib/core'
  import javascript from 'highlight.js/lib/languages/javascript'
  import 'highlight.js/styles/github.css'
  import 'highlight.js/styles/github-dark.css'

  const { t } = useI18n()
  const configManager = useConfigManager()

  // Initialize highlight.js
  hljs.registerLanguage('javascript', javascript)

  // Analysis mode options
  const analysisModes = computed(() => [
    { title: t('timeDialog.analysisModes.movetime'), value: 'movetime' },
    {
      title: t('timeDialog.analysisModes.maxThinkTime'),
      value: 'maxThinkTime',
    },
    { title: t('timeDialog.analysisModes.depth'), value: 'depth' },
    { title: t('timeDialog.analysisModes.nodes'), value: 'nodes' },
    { title: t('timeDialog.analysisModes.advanced'), value: 'advanced' },
  ])

  // A plain explanation of the currently selected mode, shown under the picker
  // so a user can tell "move time" from "max think time" without prior knowledge.
  const currentModeDescription = computed(() =>
    t(`timeDialog.modeDescriptions.${analysisMode.value}`)
  )

  // Component properties definition
  interface Props {
    modelValue: boolean
  }

  const props = defineProps<Props>()

  // Component events definition
  const emit = defineEmits<{
    'update:modelValue': [value: boolean]
    'settings-changed': [settings: AnalysisSettings]
  }>()

  // Analysis settings interface
  interface AnalysisSettings {
    movetime: number
    maxThinkTime: number
    maxDepth: number
    maxNodes: number
    analysisMode: string
    advancedScript?: string
  }

  // Reactive data
  const movetime = ref(1000)
  const maxThinkTime = ref(5000)
  const maxDepth = ref(20)
  const maxNodes = ref(1000000)
  const analysisMode = ref('movetime')
  const advancedScript = ref('')

  const advancedPlaceholder = computed(
    () =>
      `# ${t('timeDialog.advancedHint1')}\n# ${t('timeDialog.advancedHint2')}\n\n# ${t('timeDialog.advancedPlaceholder')}`
  )

  // Syntax highlighting with highlight.js (sanitized)
  const highlightedCode = computed(() => {
    if (!advancedScript.value) return ''
    try {
      const result = hljs.highlight(advancedScript.value, {
        language: 'javascript',
      })
      return result.value
    } catch (error) {
      // Fallback to plain text if highlighting fails
      return advancedScript.value
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
    }
  })

  // Very conservative sanitizer for highlight output: allow only span tags with hljs classes
  const sanitizeHighlighted = (html: string): string => {
    try {
      const container = document.createElement('div')
      container.innerHTML = html
      const isAllowedClass = (cls: string | null) => {
        if (!cls) return false
        // allow classes like "hljs", "hljs-keyword", etc.
        return cls
          .split(/\s+/)
          .every(c => c === 'hljs' || c.startsWith('hljs-'))
      }
      const walk = (node: Node): Node | null => {
        if (node.nodeType === Node.TEXT_NODE) return node
        if (node.nodeType === Node.ELEMENT_NODE) {
          const el = node as HTMLElement
          if (el.tagName.toLowerCase() !== 'span') {
            // unwrap non-span elements by returning a fragment of its children
            const fragment = document.createDocumentFragment()
            Array.from(el.childNodes).forEach(child => {
              const kept = walk(child)
              if (kept) fragment.appendChild(kept)
            })
            return fragment
          }
          // only allow class attribute and only hljs classes
          const className = el.getAttribute('class')
          if (!isAllowedClass(className)) {
            el.removeAttribute('class')
          }
          // remove all other attributes
          Array.from(el.attributes).forEach(attr => {
            if (attr.name !== 'class') el.removeAttribute(attr.name)
          })
          const clone = document.createElement('span')
          if (isAllowedClass(className))
            clone.setAttribute('class', className as string)
          Array.from(el.childNodes).forEach(child => {
            const kept = walk(child)
            if (kept) clone.appendChild(kept)
          })
          return clone
        }
        return document.createTextNode('')
      }
      const cleaned: Node[] = []
      Array.from(container.childNodes).forEach(child => {
        const kept = walk(child)
        if (kept) cleaned.push(kept)
      })
      const safeContainer = document.createElement('div')
      cleaned.forEach(n => safeContainer.appendChild(n))
      return safeContainer.innerHTML
    } catch (_) {
      // On any failure, fall back to fully escaped plain text
      return (advancedScript.value || '')
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
    }
  }

  const sanitizedHighlightedCode = computed(() =>
    sanitizeHighlighted(highlightedCode.value)
  )

  // Computed property - dialog max width based on analysis mode
  const dialogMaxWidth = computed(() => {
    return analysisMode.value === 'advanced' ? '900px' : '500px'
  })

  // Computed property - dialog visibility state
  const isVisible = computed({
    get: () => props.modelValue,
    set: (value: boolean) => emit('update:modelValue', value),
  })

  // Load settings from config file
  const loadSettings = async () => {
    try {
      await configManager.loadConfig()
      const settings = configManager.getAnalysisSettings()
      movetime.value = settings.movetime || 1000
      maxThinkTime.value = settings.maxThinkTime || 5000
      maxDepth.value = settings.maxDepth || 20
      maxNodes.value = settings.maxNodes || 1000000
      analysisMode.value = settings.analysisMode || 'movetime'
      advancedScript.value = settings.advancedScript || ''
    } catch (error) {
      console.error('Failed to load analysis settings:', error)
    }
  }

  // Save settings to config file
  const saveSettings = async () => {
    const settings = {
      movetime: movetime.value,
      maxThinkTime: maxThinkTime.value,
      maxDepth: maxDepth.value,
      maxNodes: maxNodes.value,
      analysisMode: analysisMode.value,
      advancedScript: advancedScript.value,
    }
    try {
      await configManager.updateAnalysisSettings(settings)
    } catch (error) {
      console.error('Failed to save analysis settings:', error)
    }
  }

  // Update settings and notify parent component
  const updateSettings = async () => {
    await saveSettings()
    const settings: AnalysisSettings = {
      movetime: movetime.value,
      maxThinkTime: maxThinkTime.value,
      maxDepth: maxDepth.value,
      maxNodes: maxNodes.value,
      analysisMode: analysisMode.value,
      advancedScript: advancedScript.value,
    }
    emit('settings-changed', settings)
    // console.log('TimeDialog: 设置已更新并保存:', settings);
  }

  // Reset to default values
  const resetToDefaults = async () => {
    movetime.value = 1000
    maxThinkTime.value = 5000
    maxDepth.value = 20
    maxNodes.value = 1000000
    analysisMode.value = 'movetime'
    advancedScript.value = ''
    await updateSettings()
  }

  // Clear settings — opens an in-app confirmation instead of native confirm().
  const showClearConfirm = ref(false)

  const clearSettings = () => {
    showClearConfirm.value = true
  }

  const confirmClearSettings = async () => {
    // Reset to default values
    movetime.value = 1000
    maxThinkTime.value = 5000
    maxDepth.value = 20
    maxNodes.value = 1000000
    analysisMode.value = 'movetime'
    advancedScript.value = ''

    // Clear config storage - no need to remove specific key, just save defaults
    await updateSettings()
    showClearConfirm.value = false
  }

  // Close the dialog
  const closeDialog = async () => {
    // Ensure current settings are saved when closing the dialog
    await saveSettings()
    isVisible.value = false
  }

  // Load settings after the component is mounted
  onMounted(async () => {
    await loadSettings()
  })

  // Expose methods to the parent component
  defineExpose({
    getSettings: () => ({
      movetime: movetime.value,
      maxThinkTime: maxThinkTime.value,
      maxDepth: maxDepth.value,
      maxNodes: maxNodes.value,
      analysisMode: analysisMode.value,
      advancedScript: advancedScript.value,
    }),
  })
</script>

<style lang="scss" scoped>
  .settings-container {
    padding: 20px 24px;
  }

  .field-label {
    display: block;
    font-weight: 600;
    color: rgb(var(--v-theme-on-surface));
    font-size: 14px;
    margin-bottom: 8px;
  }

  .mode-description {
    margin: 8px 0 0;
    font-size: 12px;
    line-height: 1.5;
    color: rgba(var(--v-theme-on-surface), 0.62);
  }

  .setting-block {
    display: flex;
    flex-direction: column;
  }

  .setting-input {
    max-width: 240px;
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
    font-family: var(--jb-mono, monospace);
  }

  .confirm-title {
    font-size: 16px;
    font-weight: 600;
  }

  .dialog-actions {
    padding: 12px 20px;
    border-top: 1px solid var(--jb-line, rgba(var(--v-border-color), 0.16));

    .v-btn {
      text-transform: none;
      font-weight: 500;
    }
  }

  .advanced-container {
    flex: 1;
    max-width: 800px;
  }

  .code-editor-container {
    position: relative;
    width: 100%;
  }

  .code-editor {
    font-family:
      'Consolas', 'Monaco', 'Courier New', 'JetBrains Mono', monospace !important;
    font-size: 14px !important;
    line-height: 1.5 !important;
    letter-spacing: 0.5px;

    :deep(.v-field__input) {
      font-family:
        'Consolas', 'Monaco', 'Courier New', 'JetBrains Mono', monospace !important;
      font-size: 14px !important;
      line-height: 1.5 !important;
      letter-spacing: 0.5px;
      padding: 16px !important;
      min-height: 400px !important;
      white-space: pre !important;
      tab-size: 2;
    }

    :deep(.v-field__outline) {
      border-radius: 8px !important;
    }
  }

  .editor-overlay {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    pointer-events: none;
    z-index: 1;
    padding: 16px;
    font-family:
      'Consolas', 'Monaco', 'Courier New', 'JetBrains Mono', monospace;
    font-size: 14px;
    line-height: 1.5;
    letter-spacing: 0.5px;
    white-space: pre;
    overflow: hidden;
    tab-size: 2;
  }

  .syntax-highlight {
    margin: 0;
    padding: 0;
    background: transparent;
    color: transparent;
    font-family: inherit;
    font-size: inherit;
    line-height: inherit;
    letter-spacing: inherit;
    white-space: inherit;
    tab-size: inherit;
  }

  .examples-container {
    padding: 8px 0;
  }

  .example-section {
    margin-bottom: 16px;

    &:last-child {
      margin-bottom: 0;
    }

    h4 {
      margin: 0 0 8px 0;
      font-size: 14px;
      font-weight: 600;
      color: rgb(var(--v-theme-on-surface));
    }
  }

  .code-block {
    background: rgb(var(--v-theme-surface-variant));
    border: 1px solid rgb(var(--v-theme-outline-variant));
    border-radius: 4px;
    padding: 12px;
    margin: 0;
    font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
    font-size: 12px;
    line-height: 1.4;
    color: rgb(var(--v-theme-on-surface));
    white-space: pre-wrap;
    word-wrap: break-word;
    overflow-x: auto;
  }

  .v-theme--dark .code-block {
    background: rgb(30, 30, 30);
    border-color: rgb(60, 60, 60);
  }

  .v-theme--light .code-block {
    background: rgb(248, 248, 248);
    border-color: rgb(200, 200, 200);
  }
</style>
