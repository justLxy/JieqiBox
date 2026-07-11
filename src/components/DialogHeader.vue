<template>
  <div class="jb-dialog-header">
    <div class="jb-dialog-header__accent" aria-hidden="true"></div>
    <div class="jb-dialog-header__body">
      <div class="jb-dialog-header__text">
        <div class="jb-dialog-header__title-row">
          <v-icon v-if="icon" :icon="icon" class="jb-dialog-header__icon" />
          <span class="jb-dialog-header__title">{{ title }}</span>
        </div>
        <span v-if="subtitle" class="jb-dialog-header__subtitle">{{
          subtitle
        }}</span>
      </div>
      <v-btn
        icon="mdi-close"
        variant="text"
        size="small"
        class="jb-dialog-header__close"
        :aria-label="$t('common.close')"
        @click="$emit('close')"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
  /**
   * Shared dialog header — cinnabar accent bar + title/subtitle + close button.
   * Replaces the off-brand purple gradient title bars and the bare `text-h6`
   * titles that were scattered across dialogs, so every dialog reads as part of
   * the same "Graphite & Cinnabar" instrument surface.
   */
  defineProps<{
    title: string
    subtitle?: string
    icon?: string
  }>()

  defineEmits<{
    close: []
  }>()
</script>

<style lang="scss" scoped>
  .jb-dialog-header {
    position: relative;
    background: rgb(var(--v-theme-surface));
    border-bottom: 1px solid var(--jb-line, rgba(var(--v-border-color), 0.16));
  }

  // A slim cinnabar rule along the top: the one bold accent, nothing more.
  .jb-dialog-header__accent {
    height: 3px;
    background: rgb(var(--v-theme-accent));
  }

  .jb-dialog-header__body {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    padding: 16px 20px;
  }

  .jb-dialog-header__text {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }

  .jb-dialog-header__title-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .jb-dialog-header__icon {
    color: rgb(var(--v-theme-accent));
    font-size: 22px;
  }

  .jb-dialog-header__title {
    font-size: 18px;
    font-weight: 600;
    letter-spacing: 0.01em;
    color: rgb(var(--v-theme-on-surface));
    line-height: 1.3;
  }

  .jb-dialog-header__subtitle {
    font-size: 13px;
    line-height: 1.4;
    color: rgba(var(--v-theme-on-surface), 0.62);
  }

  .jb-dialog-header__close {
    color: rgba(var(--v-theme-on-surface), 0.62);
    margin-top: -4px;
    margin-right: -8px;

    &:hover {
      color: rgb(var(--v-theme-accent));
    }
  }

  @media (max-width: 768px) {
    .jb-dialog-header__body {
      padding: 12px 16px;
    }

    .jb-dialog-header__title {
      font-size: 16px;
    }
  }
</style>
