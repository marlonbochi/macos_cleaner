<script setup lang="ts">
import { formatBytes, formatCount } from "@/composables/format";
import type { ScanProgressState } from "@/stores/scan";

defineProps<{ progress: ScanProgressState | null; running: boolean }>();
defineEmits<{ cancel: [] }>();
</script>

<template>
  <section class="card progress" aria-live="polite">
    <div class="row">
      <h2>{{ running ? $t("scan.running") : $t("scan.lastScan") }}</h2>
      <span v-if="running" class="spinner" aria-hidden="true" />
      <span class="spacer" />
      <button v-if="running" data-test="cancel" @click="$emit('cancel')">
        {{ $t("scan.cancel") }}
      </button>
    </div>
    <dl class="stats">
      <div>
        <dt>{{ $t("scan.filesSeen") }}</dt>
        <dd data-test="files-seen">{{ formatCount(progress?.filesSeen ?? 0) }}</dd>
      </div>
      <div>
        <dt>{{ $t("scan.bytesSeen") }}</dt>
        <dd data-test="bytes-seen">{{ formatBytes(progress?.bytesSeen ?? 0) }}</dd>
      </div>
      <div>
        <dt>{{ $t("scan.inaccessible") }}</dt>
        <dd>{{ formatCount(progress?.inaccessibleCount ?? 0) }}</dd>
      </div>
    </dl>
    <p
      v-if="running && progress?.currentPath"
      class="muted mono current"
      :title="progress.currentPath"
    >
      {{ progress.currentPath }}
    </p>
  </section>
</template>

<style scoped>
.stats {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 1rem;
  margin: 0.5rem 0;
}

dt {
  color: var(--text-muted);
  font-size: 0.85rem;
}

dd {
  margin: 0;
  font-size: 1.4rem;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
}

.current {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  margin: 0;
}

.spinner {
  width: 14px;
  height: 14px;
  border: 2px solid var(--bar-track);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
