<script setup lang="ts">
import type { RiskLevelDto } from "@/types/generated/RiskLevelDto";

defineProps<{ level: RiskLevelDto; title?: string }>();
</script>

<template>
  <span class="risk" :class="level.toLowerCase()" :title="title" :data-level="level">
    {{ $t(`risk.${level}`) }}
  </span>
</template>

<style scoped>
.risk {
  display: inline-block;
  font-size: 0.72rem;
  font-weight: 600;
  padding: 0.05rem 0.45rem;
  border-radius: 999px;
  border: 1px solid currentColor;
  white-space: nowrap;
  vertical-align: middle;
}

.safe {
  color: var(--success);
}

.caution {
  color: var(--warning);
}

.dangerous {
  color: var(--danger);
}

.unknown {
  color: var(--text-muted);
}
</style>
