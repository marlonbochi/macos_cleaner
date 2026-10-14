<script setup lang="ts">
import { computed } from "vue";
import { formatCount } from "@/composables/format";

const props = defineProps<{
  offset: number;
  count: number;
  total: number;
  hasPrevious: boolean;
  hasNext: boolean;
}>();
defineEmits<{ previous: []; next: [] }>();

const from = computed(() => (props.total === 0 ? 0 : props.offset + 1));
const to = computed(() => props.offset + props.count);
</script>

<template>
  <nav class="row pagination">
    <span class="muted" data-test="range">
      {{
        $t("common.pageOf", {
          from: formatCount(from),
          to: formatCount(to),
          total: formatCount(total),
        })
      }}
    </span>
    <span class="spacer" />
    <button :disabled="!hasPrevious" data-test="previous" @click="$emit('previous')">
      {{ $t("common.previous") }}
    </button>
    <button :disabled="!hasNext" data-test="next" @click="$emit('next')">
      {{ $t("common.next") }}
    </button>
  </nav>
</template>

<style scoped>
.pagination {
  margin-top: 0.75rem;
}
</style>
