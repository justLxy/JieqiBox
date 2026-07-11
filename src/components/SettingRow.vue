<template>
  <div class="setting-row" :class="{ 'setting-row--stacked': stacked }">
    <div class="setting-row__text">
      <label class="setting-row__label" :for="forId">{{ label }}</label>
      <span v-if="description" class="setting-row__description">{{
        description
      }}</span>
    </div>
    <div class="setting-row__control">
      <slot />
    </div>
  </div>
</template>

<script setup lang="ts">
  /**
   * One labeled setting: a title + explanatory description on the left and the
   * control (switch / field / select) on the right. Gives every option a plain
   * explanation so users aren't left guessing what a toggle does.
   *
   * `stacked` puts the control on its own line below the text — used for wide
   * controls like sliders and number fields.
   */
  defineProps<{
    label: string
    description?: string
    stacked?: boolean
    forId?: string
  }>()
</script>

<style lang="scss" scoped>
  .setting-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 12px 0;

    & + & {
      border-top: 1px solid var(--jb-line, rgba(var(--v-border-color), 0.12));
    }
  }

  .setting-row--stacked {
    flex-direction: column;
    align-items: stretch;
    gap: 10px;
  }

  .setting-row__text {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }

  .setting-row__label {
    font-size: 14px;
    font-weight: 600;
    color: rgb(var(--v-theme-on-surface));
    line-height: 1.3;
  }

  .setting-row__description {
    font-size: 12px;
    line-height: 1.45;
    color: rgba(var(--v-theme-on-surface), 0.6);
  }

  .setting-row__control {
    flex-shrink: 0;
    display: flex;
    align-items: center;
  }

  .setting-row--stacked .setting-row__control {
    width: 100%;
  }
</style>
