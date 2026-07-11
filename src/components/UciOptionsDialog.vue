<template>
  <v-dialog
    v-model="isVisible"
    :max-width="isMobile ? '95vw' : '800px'"
    persistent
  >
    <v-card class="uci-options-card">
      <DialogHeader
        :title="$t('uciOptions.title')"
        :subtitle="$t('uciOptions.subtitle')"
        icon="mdi-cog"
        @close="closeDialog"
      />

      <v-card-text class="options-container">
        <div v-if="isLoading" class="loading-section">
          <v-progress-circular
            indeterminate
            color="primary"
            size="48"
          ></v-progress-circular>
          <span class="loading-text">{{ $t('uciOptions.loadingText') }}</span>
        </div>

        <div v-else-if="!isEngineLoaded" class="empty-section">
          <v-icon size="64" color="grey">mdi-engine-off</v-icon>
          <p class="empty-text">{{ $t('uciOptions.noEngineLoaded') }}</p>
          <p class="empty-text-secondary">
            {{ $t('uciOptions.pleaseLoadEngineFirst') }}
          </p>
        </div>

        <div v-else-if="uciOptions.length === 0" class="empty-section">
          <v-icon size="64" color="grey">mdi-cog-off</v-icon>
          <p class="empty-text">{{ $t('uciOptions.noOptionsAvailable') }}</p>
          <v-btn
            color="primary"
            @click="refreshOptions"
            :loading="isLoading"
            :disabled="areOptionsLocked"
            size="large"
            class="action-btn"
          >
            {{ $t('uciOptions.refreshOptions') }}
          </v-btn>
        </div>

        <div v-else class="options-list">
          <div
            v-for="option in uciOptions"
            :key="option.name"
            class="option-item"
          >
            <!-- Conditional rendering for controls -->
            <div>
              <!-- 数值类型选项 (spin) -->
              <div v-if="option.type === 'spin'" class="option-row spin-option">
                <div class="option-header">
                  <div class="option-label-group">
                    <label class="option-label">{{
                      getOptionLabel(option.name)
                    }}</label>
                    <span
                      v-if="getOptionLabel(option.name) !== option.name"
                      class="option-key"
                      >{{ option.name }}</span
                    >
                  </div>
                  <span class="option-range"
                    >{{ $t('uciOptions.range') }}: {{ option.min }} -
                    {{ option.max }}</span
                  >
                </div>
                <div class="option-controls">
                  <v-text-field
                    v-model.number="option.currentValue"
                    :min="option.min"
                    :max="option.max"
                    :disabled="areOptionsLocked"
                    type="number"
                    variant="outlined"
                    density="compact"
                    hide-details
                    class="number-input"
                    @update:model-value="updateOption(option.name, $event)"
                  ></v-text-field>
                </div>
              </div>

              <!-- 布尔类型选项 (check) -->
              <div
                v-else-if="option.type === 'check'"
                class="option-row check-option"
              >
                <div class="d-flex justify-space-between align-center">
                  <div class="option-label-group">
                    <label class="option-label">{{
                      getOptionLabel(option.name)
                    }}</label>
                    <span
                      v-if="getOptionLabel(option.name) !== option.name"
                      class="option-key"
                      >{{ option.name }}</span
                    >
                  </div>
                  <v-switch
                    v-model="option.currentValue as boolean"
                    :disabled="areOptionsLocked"
                    color="primary"
                    class="option-switch flex-grow-0"
                    hide-details
                    @update:model-value="
                      updateOption(option.name, $event ?? false)
                    "
                  ></v-switch>
                </div>
              </div>

              <!-- 下拉选择类型选项 (combo) -->
              <div
                v-else-if="option.type === 'combo'"
                class="option-row combo-option"
              >
                <div class="option-label-group">
                  <label class="option-label">{{
                    getOptionLabel(option.name)
                  }}</label>
                  <span
                    v-if="getOptionLabel(option.name) !== option.name"
                    class="option-key"
                    >{{ option.name }}</span
                  >
                </div>
                <v-select
                  v-model="option.currentValue as string"
                  :items="option.vars"
                  :disabled="areOptionsLocked"
                  variant="outlined"
                  density="compact"
                  hide-details
                  class="option-select"
                  @update:model-value="updateOption(option.name, $event || '')"
                ></v-select>
              </div>

              <!-- 字符串类型选项 (string) -->
              <div
                v-else-if="option.type === 'string'"
                class="option-row string-option"
              >
                <div class="option-label-group">
                  <label class="option-label">{{
                    getOptionLabel(option.name)
                  }}</label>
                  <span
                    v-if="getOptionLabel(option.name) !== option.name"
                    class="option-key"
                    >{{ option.name }}</span
                  >
                </div>
                <v-text-field
                  v-model="option.currentValue as string"
                  :disabled="areOptionsLocked"
                  variant="outlined"
                  density="compact"
                  hide-details
                  class="option-input"
                  @update:model-value="updateOption(option.name, $event || '')"
                ></v-text-field>
              </div>

              <!-- 按钮类型选项 (button) -->
              <div
                v-else-if="option.type === 'button'"
                class="option-row button-option"
              >
                <div class="d-flex justify-space-between align-center">
                  <div class="option-label-group">
                    <label class="option-label">{{
                      getOptionLabel(option.name)
                    }}</label>
                    <span
                      v-if="getOptionLabel(option.name) !== option.name"
                      class="option-key"
                      >{{ option.name }}</span
                    >
                  </div>
                  <v-btn
                    color="primary"
                    variant="outlined"
                    class="execute-btn"
                    :loading="isApplyingOptions"
                    :disabled="areOptionsLocked"
                    @click="executeButtonOption(option.name)"
                  >
                    {{ $t('uciOptions.execute') }}
                  </v-btn>
                </div>
              </div>
            </div>

            <div
              v-if="getOptionDescription(option.name)"
              class="option-description"
            >
              <v-icon size="small" class="description-icon"
                >mdi-information-outline</v-icon
              >
              <span class="description-text">{{
                getOptionDescription(option.name)
              }}</span>
            </div>
          </div>
        </div>
      </v-card-text>

      <v-card-actions class="dialog-actions">
        <div class="actions-container">
          <div class="action-buttons">
            <v-btn
              color="grey"
              @click="resetToDefaults"
              :disabled="areOptionsLocked"
              size="small"
              class="action-btn"
            >
              {{ $t('uciOptions.resetToDefaults') }}
            </v-btn>
            <v-btn
              color="primary"
              @click="refreshOptions"
              :loading="isLoading"
              :disabled="areOptionsLocked"
              size="small"
              class="action-btn"
            >
              {{ $t('uciOptions.refreshOptions') }}
            </v-btn>
            <v-btn
              color="grey"
              @click="clearSettings"
              :disabled="areOptionsLocked"
              size="small"
              class="action-btn"
            >
              {{ $t('uciOptions.clearSettings') }}
            </v-btn>
          </div>
          <v-btn
            color="grey"
            @click="closeDialog"
            :loading="isApplyingOptions"
            :disabled="isBusy"
            size="small"
            class="close-action-btn"
          >
            {{ $t('common.close') }}
          </v-btn>
        </div>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<script setup lang="ts">
  import { ref, computed, onMounted, watch, inject } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { useConfigManager } from '../composables/useConfigManager'
  import DialogHeader from './DialogHeader.vue'

  // UCI option interface definition
  interface UciOption {
    name: string
    type: 'spin' | 'check' | 'combo' | 'string' | 'button'
    defaultValue: string | number | boolean
    currentValue: string | number | boolean
    min?: number
    max?: number
    vars?: string[]
  }

  type UciOptionValue = string | number | boolean

  // Component properties definition
  interface Props {
    modelValue: boolean
    engineId?: string
  }

  const props = withDefaults(defineProps<Props>(), {
    engineId: 'default',
  })

  // Component events definition
  const emit = defineEmits<{
    'update:modelValue': [value: boolean]
  }>()

  // Inject engine state
  const { t } = useI18n()
  const engineState = inject('engine-state') as any
  const {
    isEngineLoaded,
    uciOptionsText,
    isApplyingOptions,
    isThinking,
    isPondering,
  } = engineState

  // Configuration manager
  const configManager = useConfigManager()

  // Reactive data
  const isLoading = ref(false)
  const uciOptions = ref<UciOption[]>([])
  const originalOptions = ref<Record<string, string | number | boolean>>({})
  const isBusy = computed(() => isLoading.value || isApplyingOptions.value)
  const areOptionsLocked = computed(
    () => isBusy.value || isThinking.value || isPondering.value
  )

  // Detect if the device is mobile
  const isMobile = computed(() => {
    return window.innerWidth <= 768
  })

  // Computed property - dialog visibility state
  const isVisible = computed({
    get: () => props.modelValue,
    set: (value: boolean) => emit('update:modelValue', value),
  })

  // Engine path hash for config storage
  const enginePathHash = computed(() => {
    // THIS IS THE KEY CHANGE. Use the engine's unique ID.
    if (!props.engineId) return 'default'
    return props.engineId
  })

  // Function to parse UCI options text
  const parseUciOptions = (uciText: string): UciOption[] => {
    const options = new Map<string, UciOption>()
    const lines = uciText.split('\n')

    lines.forEach(line => {
      const trimmedLine = line.trim()
      if (trimmedLine.startsWith('option name ')) {
        const option = parseUciOptionLine(trimmedLine)
        if (option) {
          options.set(option.name, option)
        }
      }
    })

    return [...options.values()]
  }

  // Function to parse a single UCI option line
  const parseUciOptionLine = (line: string): UciOption | null => {
    const nameMatch = line.match(/option name (.+?) type (.+)/)
    if (!nameMatch) return null

    const name = nameMatch[1]
    const typeMatch = line.match(/type (\w+)/)
    if (!typeMatch) return null

    const type = typeMatch[1] as UciOption['type']

    // Create a basic option object
    const option: UciOption = {
      name,
      type,
      defaultValue: '',
      currentValue: '',
    }

    // Parse specific parameters based on type
    switch (type) {
      case 'spin':
        const defaultSpinMatch = line.match(/default (-?\d+)/)
        const minMatch = line.match(/min (-?\d+)/)
        const maxMatch = line.match(/max (-?\d+)/)

        option.defaultValue = defaultSpinMatch
          ? parseInt(defaultSpinMatch[1])
          : 0
        option.min = minMatch ? parseInt(minMatch[1]) : 0
        option.max = maxMatch ? parseInt(maxMatch[1]) : 100
        break

      case 'check':
        const defaultCheckMatch = line.match(/default (true|false)/)
        option.defaultValue = defaultCheckMatch
          ? defaultCheckMatch[1] === 'true'
          : false
        break

      case 'combo':
        const defaultComboMatch = line.match(/default (.+?)(?=\s+var\s|$)/)
        const varsMatch = line.match(/var (.+)/)

        option.defaultValue = defaultComboMatch ? defaultComboMatch[1] : ''
        option.vars = varsMatch ? varsMatch[1].split(' var ') : []
        break

      case 'string':
        const defaultStringMatch = line.match(/default\s*(.*)$/)
        option.defaultValue = defaultStringMatch ? defaultStringMatch[1] : ''
        break

      case 'button':
        option.defaultValue = ''
        break
    }

    // Set current value to the default value
    option.currentValue = option.defaultValue

    return option
  }

  // Saved values describe the engine's active configuration after startup.
  // They are applied by useUciEngine during loading, so opening this dialog
  // must only reflect them and never replay large options such as Hash.
  const loadSavedOptionValues = () => {
    const savedOptions = configManager.getUciOptions(enginePathHash.value)

    uciOptions.value.forEach(option => {
      if (savedOptions[option.name] !== undefined) {
        option.currentValue = normalizeOptionValue(
          option,
          savedOptions[option.name]
        )
      }
    })
  }

  const normalizeOptionValue = (
    option: UciOption,
    value: unknown
  ): UciOptionValue => {
    switch (option.type) {
      case 'spin': {
        const numericValue = Number(value)
        return Number.isFinite(numericValue)
          ? numericValue
          : (option.defaultValue as number)
      }
      case 'check':
        return value === true || String(value).trim().toLowerCase() === 'true'
      case 'combo':
      case 'string':
        return String(value)
      case 'button':
        return ''
    }
  }

  const areOptionValuesEqual = (
    option: UciOption,
    left: unknown,
    right: unknown
  ) =>
    normalizeOptionValue(option, left) === normalizeOptionValue(option, right)

  const captureOriginalOptions = () => {
    originalOptions.value = {}
    uciOptions.value.forEach(option => {
      if (option.type !== 'button') {
        originalOptions.value[option.name] = option.currentValue
      }
    })
  }

  // Persist only overrides. Saving all engine defaults causes every option to
  // be replayed during the next startup, including expensive Hash allocation.
  const saveOptionsToStorage = async () => {
    const savedOptions = configManager.getUciOptions(enginePathHash.value)
    const optionsByName = new Map(
      uciOptions.value.map(option => [option.name, option])
    )
    const optionsToSave = Object.fromEntries(
      Object.entries(savedOptions).filter(([name]) => {
        const option = optionsByName.get(name)
        return !option || option.type === 'button'
      })
    ) as Record<string, UciOptionValue>

    uciOptions.value.forEach(option => {
      if (
        option.type !== 'button' &&
        !areOptionValuesEqual(option, option.currentValue, option.defaultValue)
      ) {
        optionsToSave[option.name] = option.currentValue
      }
    })

    const savedKeys = Object.keys(savedOptions)
    const nextKeys = Object.keys(optionsToSave)
    const isUnchanged =
      savedKeys.length === nextKeys.length &&
      nextKeys.every(key => {
        const option = uciOptions.value.find(item => item.name === key)
        return option
          ? areOptionValuesEqual(option, savedOptions[key], optionsToSave[key])
          : savedOptions[key] === optionsToSave[key]
      })

    if (isUnchanged) return
    await configManager.updateUciOptions(enginePathHash.value, optionsToSave)
  }

  const applyOptionChanges = async (
    changes: Array<{
      name: string
      value?: string | number | boolean
    }>
  ) => {
    if (changes.length === 0) return
    await engineState.applyUciOptions(changes)
  }

  // Function to update an option's value
  const updateOption = (name: string, value: UciOptionValue) => {
    const option = uciOptions.value.find(opt => opt.name === name)
    if (option) {
      option.currentValue = normalizeOptionValue(option, value)
    }
  }

  // Function to execute a button-type option
  const executeButtonOption = async (name: string) => {
    if (areOptionsLocked.value) return

    try {
      await applyOptionChanges([{ name }])
    } catch (error) {
      console.error('Failed to execute UCI option:', error)
      alert(t('uciOptions.applyFailed'))
    }
  }

  // Function to reset to default values
  const resetToDefaults = async () => {
    if (areOptionsLocked.value) return

    const changes = uciOptions.value
      .filter(option => option.type !== 'button')
      .map(option => ({ name: option.name, value: option.defaultValue }))

    try {
      await applyOptionChanges(changes)
      uciOptions.value.forEach(option => {
        if (option.type !== 'button') option.currentValue = option.defaultValue
      })
      await configManager.clearUciOptions(enginePathHash.value)
      captureOriginalOptions()
    } catch (error) {
      console.error('Failed to reset UCI options:', error)
      alert(t('uciOptions.applyFailed'))
    }
  }

  // Function to refresh UCI options
  const refreshOptions = async () => {
    if (!isEngineLoaded.value) {
      alert(t('uciOptions.noEngineLoaded'))
      return
    }
    if (areOptionsLocked.value) return

    isLoading.value = true

    try {
      const optionText = uciOptionsText.value.trim()
        ? uciOptionsText.value
        : await engineState.requestUciOptions()
      const options = parseUciOptions(optionText)
      uciOptions.value = options
      loadSavedOptionValues()
      captureOriginalOptions()
    } catch (error) {
      console.error('Failed to load UCI options:', error)
      alert(t('uciOptions.loadFailed'))
    } finally {
      isLoading.value = false
    }
  }

  // Function to close the dialog
  const closeDialog = async () => {
    if (isBusy.value) return

    try {
      const changes = uciOptions.value
        .filter(
          option =>
            option.type !== 'button' &&
            !areOptionValuesEqual(
              option,
              originalOptions.value[option.name],
              option.currentValue
            )
        )
        .map(option => ({ name: option.name, value: option.currentValue }))

      if (changes.length > 0 && (isThinking.value || isPondering.value)) {
        alert(t('uciOptions.engineBusy'))
        return
      }

      await applyOptionChanges(changes)

      // Always normalize legacy configurations which may have persisted every
      // default option, while avoiding a write when the effective overrides
      // are already unchanged.
      await saveOptionsToStorage()
      captureOriginalOptions()
      isVisible.value = false
    } catch (error) {
      console.error('Failed to save UCI options on close:', error)
      alert(t('uciOptions.applyFailed'))
    }
  }

  // Watch the dialog's open state
  watch(isVisible, newVal => {
    if (newVal && isEngineLoaded.value) {
      void refreshOptions()
    }
  })

  // Initialization after component is mounted
  onMounted(async () => {
    // Load configuration first
    await configManager.loadConfig()

    // If the engine is already loaded, get options immediately
    if (isVisible.value && isEngineLoaded.value) void refreshOptions()
  })

  // Resolve a friendly, localized display name for a known engine option. Falls
  // back to the raw engine key (e.g. "MultiPV") when no translation exists, so
  // custom/unknown options still show something meaningful.
  const getOptionLabel = (optionName: string): string => {
    const fullKey = `uciOptions.optionNames.${optionName}`
    const label = t(fullKey)
    return label !== fullKey ? label : optionName
  }

  // Function to get option description from i18n
  const getOptionDescription = (optionName: string): string => {
    // Construct the full key path to the specific translation.
    // For example: 'uciOptions.optionDescriptions.Threads' or 'uciOptions.optionDescriptions.Debug Log File'
    const fullKey = `uciOptions.optionDescriptions.${optionName}`

    // Use the t() function to get the final translated string directly.
    const description = t(fullKey)

    // IMPORTANT: If the t() function cannot find a corresponding translation, it will return the key itself.
    // We need to check if the returned value is equal to the key we provided to determine if the translation exists.
    // If they are equal, it means the translation was not found, so we return an empty string, which will cause the v-if to hide the element.
    return description !== fullKey ? description : ''
  }

  // Function to clear settings
  const clearSettings = async () => {
    if (
      !areOptionsLocked.value &&
      confirm(t('uciOptions.confirmClearSettings'))
    ) {
      await resetToDefaults()
    }
  }

  // Expose methods to the parent component
  defineExpose({
    refreshOptions,
    resetToDefaults,
    clearSettings,
  })
</script>

<style lang="scss" scoped>
  .uci-options-card {
    border-radius: var(--jb-radius);
    overflow: hidden;
  }

  .options-container {
    max-height: 70vh;
    overflow-y: auto;
    padding: 16px;
  }

  .loading-section {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    padding: 40px 20px;

    .loading-text {
      color: rgb(var(--v-theme-on-surface));
      font-size: 16px;
      text-align: center;
    }
  }

  .empty-section {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 20px;
    padding: 40px 20px;
    color: rgb(var(--v-theme-on-surface));

    .empty-text {
      margin: 0;
      font-size: 16px;
      text-align: center;
      line-height: 1.5;
      color: rgb(var(--v-theme-on-surface));
    }

    .empty-text-secondary {
      margin: 0;
      font-size: 14px;
      text-align: center;
      line-height: 1.4;
      color: rgb(var(--v-theme-on-surface));
      opacity: 0.7;
    }

    .action-btn {
      min-width: 120px;
    }
  }

  .options-list {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .option-item {
    border: 1px solid var(--jb-line, rgba(var(--v-border-color), 0.16));
    border-radius: var(--jb-radius);
    padding: 16px;
    background: rgb(var(--v-theme-surface));
    transition:
      border-color 0.2s ease,
      background 0.2s ease;

    &:hover {
      border-color: rgba(var(--v-theme-accent), 0.5);
    }
  }

  .dialog-actions {
    background: rgb(var(--v-theme-surface));
    border-top: 1px solid var(--jb-line, rgba(var(--v-border-color), 0.16));
  }

  .option-row {
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-height: 48px;
  }

  .option-header {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .option-label-group {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .option-label {
    font-weight: 600;
    color: rgb(var(--v-theme-on-surface));
    font-size: 16px;
    line-height: 1.4;
  }

  // The raw engine key, shown small and mono under the friendly name so power
  // users can still map it to the UCI protocol.
  .option-key {
    font-family: var(--jb-mono, monospace);
    font-size: 11px;
    line-height: 1.2;
    color: rgba(var(--v-theme-on-surface), 0.45);
  }

  .option-range {
    font-family: var(--jb-mono, monospace);
    font-variant-numeric: tabular-nums;
    font-size: 12px;
    color: rgb(var(--v-theme-on-surface));
    font-weight: 400;
    opacity: 0.7;
  }

  .option-controls {
    display: flex;
    flex-direction: column;
    gap: 12px;
    width: 100%;
  }

  .number-input {
    max-width: 120px;
    align-self: flex-start;

    :deep(input) {
      font-family: var(--jb-mono, monospace);
      font-variant-numeric: tabular-nums;
    }
  }

  .option-switch {
    align-self: flex-end;
    margin-top: 8px;
  }

  .option-select,
  .option-input {
    width: 100%;
  }

  .execute-btn {
    align-self: flex-end;
    margin-top: 8px;
    min-width: 100px;
  }

  // A quiet neutral note, marked with an accent rule on the left — reads as
  // "help text", not an error box (the old primary tint is cinnabar in light
  // mode and looked like a warning).
  .option-description {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    font-size: 13px;
    line-height: 1.5;
    margin-top: 12px;
    padding: 10px 12px;
    background-color: rgba(var(--v-theme-on-surface), 0.04);
    border-left: 3px solid rgba(var(--v-theme-accent), 0.5);
    border-radius: var(--jb-radius-sm);
    color: rgba(var(--v-theme-on-surface), 0.75);

    .description-icon {
      color: rgba(var(--v-theme-on-surface), 0.5);
      margin-top: 2px;
    }

    .description-text {
      flex: 1;
    }
  }

  .dialog-actions {
    padding: 16px 20px;
    background: rgb(var(--v-theme-surface));

    .actions-container {
      display: flex;
      flex-direction: column;
      gap: 12px;
      width: 100%;
    }

    .action-buttons {
      display: flex;
      flex-wrap: wrap;
      gap: 8px;
      justify-content: center;
    }

    .action-btn {
      text-transform: none;
      font-weight: 500;
      font-size: 14px;
      min-width: 80px;
    }

    .close-action-btn {
      text-transform: none;
      font-weight: 500;
      font-size: 14px;
      align-self: center;
      min-width: 100px;
    }
  }

  @media (max-width: 768px) {
    .options-container {
      max-height: 60vh;
      padding: 12px;
    }

    .option-item {
      padding: 12px;
      border-radius: 8px;
    }

    .option-label {
      font-size: 15px;
    }

    .dialog-actions {
      padding: 12px 16px;

      .action-buttons {
        gap: 6px;
      }

      .action-btn {
        font-size: 13px;
        min-width: 70px;
        padding: 8px 12px;
      }

      .close-action-btn {
        font-size: 13px;
        min-width: 80px;
        padding: 8px 16px;
      }
    }

    .dialog-title {
      padding: 12px 16px;

      .title-text {
        font-size: 16px;
      }
    }

    .loading-section,
    .empty-section {
      padding: 30px 16px;
    }

    .loading-text,
    .empty-text {
      font-size: 14px;
    }
  }

  // Extra small screen optimization
  @media (max-width: 480px) {
    .options-container {
      max-height: 55vh;
      padding: 8px;
    }

    .option-item {
      padding: 10px;
    }

    .option-label {
      font-size: 14px;
    }

    .dialog-actions {
      padding: 10px 12px;

      .action-btn {
        font-size: 12px;
        min-width: 60px;
        padding: 6px 10px;
      }

      .close-action-btn {
        font-size: 12px;
        min-width: 70px;
        padding: 6px 12px;
      }
    }

    .dialog-title {
      padding: 10px 12px;

      .title-text {
        font-size: 15px;
      }
    }
  }

  // Scrollbar styles
  .options-container::-webkit-scrollbar {
    width: 4px;
  }

  .options-container::-webkit-scrollbar-track {
    background: rgb(var(--v-theme-surface-variant));
    border-radius: 2px;
  }

  .options-container::-webkit-scrollbar-thumb {
    background: rgb(var(--v-theme-outline));
    border-radius: 2px;

    &:hover {
      background: rgb(var(--v-theme-outline-variant));
    }
  }
</style>
