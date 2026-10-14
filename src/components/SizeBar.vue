<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{ value: number; max: number }>();

/** Percentage of the largest value, with a visible minimum for non-zero sizes. */
const percent = computed(() => {
  if (props.max <= 0 || props.value <= 0) return 0;
  return Math.max(1, Math.min(100, (props.value / props.max) * 100));
});
</script>

<template>
  <div class="size-bar" role="presentation">
    <div class="fill" :style="{ width: `${percent}%` }" />
  </div>
</template>

<style scoped>
.size-bar {
  height: 6px;
  border-radius: 3px;
  background: var(--bar-track);
  overflow: hidden;
  min-width: 80px;
}

.fill {
  height: 100%;
  background: var(--bar);
}
</style>
