<template>
  <v-dialog v-model="isDialogVisible" max-width="620px" scrollable>
    <v-card class="settings-card">
      <DialogHeader
        :title="$t('interfaceSettings.title')"
        :subtitle="$t('interfaceSettings.subtitle')"
        icon="mdi-tune-variant"
        @close="closeDialog"
      />

      <v-card-text class="settings-body">
        <!-- Appearance -->
        <section class="settings-section">
          <h3 class="settings-section__title">
            {{ $t('interfaceSettings.groups.appearance') }}
          </h3>
          <SettingRow
            :label="$t('interfaceSettings.darkMode')"
            :description="$t('interfaceSettings.descriptions.darkMode')"
          >
            <v-switch
              v-model="darkMode"
              color="primary"
              hide-details
              density="compact"
            />
          </SettingRow>
          <SettingRow
            :label="$t('interfaceSettings.pieceStyle')"
            :description="$t('interfaceSettings.descriptions.pieceStyle')"
          >
            <v-select
              v-model="pieceStyle"
              :items="pieceStyleItems"
              color="primary"
              variant="outlined"
              density="compact"
              hide-details
              class="inline-select"
            />
          </SettingRow>
          <SettingRow
            :label="$t('interfaceSettings.showAnimations')"
            :description="$t('interfaceSettings.descriptions.showAnimations')"
          >
            <v-switch
              v-model="showAnimations"
              color="primary"
              hide-details
              density="compact"
            />
          </SettingRow>
        </section>

        <!-- Board -->
        <section class="settings-section">
          <h3 class="settings-section__title">
            {{ $t('interfaceSettings.groups.board') }}
          </h3>
          <SettingRow
            :label="$t('interfaceSettings.showCoordinates')"
            :description="$t('interfaceSettings.descriptions.showCoordinates')"
          >
            <v-switch
              v-model="showCoordinates"
              color="primary"
              hide-details
              density="compact"
            />
          </SettingRow>
          <SettingRow
            :label="$t('interfaceSettings.showArrows')"
            :description="$t('interfaceSettings.descriptions.showArrows')"
          >
            <v-switch
              v-model="showArrows"
              color="primary"
              hide-details
              density="compact"
            />
          </SettingRow>
          <SettingRow
            :label="$t('interfaceSettings.showChineseNotation')"
            :description="
              $t('interfaceSettings.descriptions.showChineseNotation')
            "
          >
            <v-switch
              v-model="showChineseNotation"
              color="primary"
              hide-details
              density="compact"
            />
          </SettingRow>
        </section>

        <!-- Analysis display -->
        <section class="settings-section">
          <h3 class="settings-section__title">
            {{ $t('interfaceSettings.groups.analysis') }}
          </h3>
          <SettingRow
            :label="$t('interfaceSettings.parseUciInfo')"
            :description="$t('interfaceSettings.descriptions.parseUciInfo')"
          >
            <v-switch
              v-model="parseUciInfo"
              color="primary"
              hide-details
              density="compact"
            />
          </SettingRow>
          <SettingRow
            :label="$t('interfaceSettings.showEvaluationBar')"
            :description="$t('interfaceSettings.descriptions.showEvaluationBar')"
          >
            <v-switch
              v-model="showEvaluationBar"
              color="primary"
              hide-details
              density="compact"
            />
          </SettingRow>
          <SettingRow
            :label="$t('interfaceSettings.showPositionChart')"
            :description="$t('interfaceSettings.descriptions.showPositionChart')"
          >
            <v-switch
              v-model="showPositionChart"
              color="primary"
              hide-details
              density="compact"
            />
          </SettingRow>
          <SettingRow
            :label="$t('interfaceSettings.showLuckIndex')"
            :description="$t('interfaceSettings.descriptions.showLuckIndex')"
          >
            <v-switch
              v-model="showLuckIndex"
              color="primary"
              hide-details
              density="compact"
            />
          </SettingRow>
        </section>

        <!-- Sound -->
        <section class="settings-section">
          <h3 class="settings-section__title">
            {{ $t('interfaceSettings.groups.sound') }}
          </h3>
          <SettingRow
            :label="$t('interfaceSettings.enableSoundEffects')"
            :description="
              $t('interfaceSettings.descriptions.enableSoundEffects')
            "
          >
            <v-switch
              v-model="enableSoundEffects"
              color="primary"
              hide-details
              density="compact"
            />
          </SettingRow>
          <SettingRow
            :label="$t('interfaceSettings.soundVolume')"
            :description="$t('interfaceSettings.descriptions.soundVolume')"
            stacked
          >
            <v-slider
              v-model="soundVolume"
              :disabled="!enableSoundEffects"
              :min="0"
              :max="100"
              :step="5"
              thumb-label
              color="primary"
              hide-details
            >
              <template #append>
                <span class="volume-readout">{{ soundVolume }}%</span>
              </template>
            </v-slider>
          </SettingRow>
        </section>

        <!-- Data & engine -->
        <section class="settings-section">
          <h3 class="settings-section__title">
            {{ $t('interfaceSettings.groups.data') }}
          </h3>
          <SettingRow
            :label="$t('interfaceSettings.autosave')"
            :description="$t('interfaceSettings.descriptions.autosave')"
          >
            <v-switch
              v-model="autosave"
              color="primary"
              hide-details
              density="compact"
            />
          </SettingRow>
          <SettingRow
            :label="$t('interfaceSettings.useNewFenFormat')"
            :description="$t('interfaceSettings.descriptions.useNewFenFormat')"
          >
            <v-switch
              v-model="useNewFenFormat"
              color="primary"
              hide-details
              density="compact"
            />
          </SettingRow>
          <SettingRow
            :label="$t('interfaceSettings.engineLogLineLimit')"
            :description="
              $t('interfaceSettings.descriptions.engineLogLineLimit')
            "
          >
            <v-text-field
              v-model.number="engineLogLineLimit"
              type="number"
              min="50"
              max="1000"
              step="50"
              color="primary"
              variant="outlined"
              density="compact"
              hide-details
              class="inline-number"
            />
          </SettingRow>
          <SettingRow
            :label="$t('interfaceSettings.validationTimeout')"
            :description="$t('interfaceSettings.descriptions.validationTimeout')"
          >
            <v-text-field
              v-model.number="validationTimeout"
              type="number"
              min="1000"
              max="30000"
              step="1000"
              color="primary"
              variant="outlined"
              density="compact"
              hide-details
              class="inline-number"
            />
          </SettingRow>
        </section>
      </v-card-text>

      <v-card-actions class="settings-actions">
        <v-spacer></v-spacer>
        <v-btn color="primary" variant="flat" @click="closeDialog">{{
          $t('common.close')
        }}</v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<script setup lang="ts">
  import { computed } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { useInterfaceSettings } from '@/composables/useInterfaceSettings'
  import DialogHeader from './DialogHeader.vue'
  import SettingRow from './SettingRow.vue'

  const { t } = useI18n()
  const isDialogVisible = defineModel<boolean>()

  // Get interface settings from shared state
  const {
    showCoordinates,
    parseUciInfo,
    showAnimations,
    showPositionChart,
    showEvaluationBar,
    darkMode,
    autosave,
    useNewFenFormat,
    engineLogLineLimit,
    validationTimeout,
    showChineseNotation,
    showLuckIndex,
    showArrows,
    enableSoundEffects,
    soundVolume,
    pieceStyle,
  } = useInterfaceSettings()

  const pieceStyleItems = computed(() => [
    {
      title: t('interfaceSettings.pieceStyles.default'),
      value: 'default',
    },
    {
      title: t('interfaceSettings.pieceStyles.internationalized'),
      value: 'internationalized',
    },
  ])

  const closeDialog = () => {
    isDialogVisible.value = false
  }
</script>

<style lang="scss" scoped>
  .settings-card {
    border-radius: var(--jb-radius);
    overflow: hidden;
  }

  .settings-body {
    max-height: 70vh;
    padding: 4px 24px 8px;
  }

  .settings-section {
    padding: 8px 0 4px;

    & + & {
      margin-top: 8px;
      border-top: 1px solid var(--jb-line, rgba(var(--v-border-color), 0.16));
    }
  }

  .settings-section__title {
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: rgb(var(--v-theme-accent));
    margin: 12px 0 2px;
  }

  .inline-select {
    min-width: 180px;
  }

  .inline-number {
    max-width: 130px;
  }

  .volume-readout {
    font-family: var(--jb-mono, monospace);
    font-variant-numeric: tabular-nums;
    font-size: 13px;
    min-width: 42px;
    text-align: right;
    color: rgb(var(--v-theme-on-surface));
  }

  .settings-actions {
    padding: 12px 20px;
    border-top: 1px solid var(--jb-line, rgba(var(--v-border-color), 0.16));
  }

  @media (max-width: 768px) {
    .settings-body {
      padding: 4px 16px 8px;
    }

    .inline-select {
      min-width: 150px;
    }
  }
</style>
