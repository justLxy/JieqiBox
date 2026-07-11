<template>
  <v-tooltip :text="label" location="bottom" open-delay="200">
    <template #activator="{ props: tooltipProps }">
      <!-- A disabled v-btn swallows mouse events, so the tooltip would never
           show. Wrap in a span that still receives hover when disabled. -->
      <span class="toolbar-button__wrap" v-bind="tooltipProps">
        <v-btn
          :icon="icon"
          size="small"
          variant="text"
          class="tool-icon"
          :loading="loading"
          :disabled="disabled"
          @click="$emit('click')"
        />
      </span>
    </template>
  </v-tooltip>
</template>

<script setup lang="ts">
  /**
   * Toolbar icon button with a proper styled tooltip. Replaces the native
   * `:title` attribute (slow to appear, unstyled by the OS) so every toolbar
   * icon announces its name on hover in a consistent in-app tooltip.
   */
  defineProps<{
    icon: string
    label: string
    disabled?: boolean
    loading?: boolean
  }>()

  defineEmits<{
    click: []
  }>()
</script>

<style lang="scss" scoped>
  // Keep the wrap inline so buttons stay laid out exactly as before.
  .toolbar-button__wrap {
    display: inline-flex;
  }

  // Quiet, uniform icon strip: neutral ink at rest, cinnabar on hover.
  .tool-icon {
    color: rgba(var(--v-theme-on-surface), 0.62);
    transition:
      color 0.15s ease,
      background-color 0.15s ease;

    &:hover:not(.v-btn--disabled) {
      color: rgb(var(--v-theme-accent));
    }
  }
</style>
