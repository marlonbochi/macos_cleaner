<script setup lang="ts">
import { computed, onMounted, watch } from "vue";
import { useRouter } from "vue-router";
import AnalysisPanel from "@/components/AnalysisPanel.vue";
import FileTable from "@/components/FileTable.vue";
import RiskBadge from "@/components/RiskBadge.vue";
import { formatBytes, formatDateTime } from "@/composables/format";
import { useAiSettingsStore } from "@/stores/aiSettings";
import { useAnalysisStore } from "@/stores/analysis";
import { useCatalogStore } from "@/stores/catalog";
import { useDiagnosisStore } from "@/stores/diagnosis";
import { useScanStore } from "@/stores/scan";
import type { RiskLevelDto } from "@/types/generated/RiskLevelDto";

const scan = useScanStore();
const ai = useAiSettingsStore();
const diagnosis = useDiagnosisStore();
const analysis = useAnalysisStore();
const catalog = useCatalogStore();
const router = useRouter();

const LEVELS: RiskLevelDto[] = ["Safe", "Caution", "Dangerous", "Unknown"];

const home = computed(() => scan.defaults?.home ?? null);
const finishedScan = computed(() => (scan.hasCatalog ? scan.current : null));
const files = computed(() => diagnosis.items.map((i) => i.file));
const model = computed(() => ai.saved?.model ?? "");

onMounted(async () => {
  if (!ai.saved) await ai.loadSaved();
  if (!scan.defaults) await scan.init();
});

watch(
  () => finishedScan.value?.id,
  (id) => {
    if (id !== undefined && !diagnosis.running) void diagnosis.load(id);
  },
  { immediate: true },
);

function run(force: boolean): void {
  if (finishedScan.value) void diagnosis.run(finishedScan.value.id, force);
}

/** Folders are browsed in the largest-items list. */
async function openFolder(path: string): Promise<void> {
  if (!finishedScan.value) return;
  await catalog.open(finishedScan.value.id);
  await catalog.openFolder(path);
  await router.push({ name: "files" });
}
</script>

<template>
  <div>
    <header class="row">
      <div>
        <h1>{{ $t("diagnosis.title") }}</h1>
        <p class="muted">{{ $t("diagnosis.subtitle") }}</p>
        <p v-if="finishedScan" class="muted">
          {{ $t("scan.lastScanAt", { date: formatDateTime(finishedScan.finishedAt) }) }}
        </p>
      </div>
      <span class="spacer" />
      <button
        v-if="ai.aiEnabled && finishedScan && diagnosis.items.length > 0"
        :class="{ primary: diagnosis.pending > 0 }"
        data-test="run"
        :disabled="diagnosis.running"
        @click="run(diagnosis.pending === 0)"
      >
        {{ diagnosis.pending > 0 ? $t("diagnosis.run") : $t("diagnosis.again") }}
      </button>
    </header>

    <section v-if="!ai.aiEnabled" class="card" data-test="needs-ai">
      <p>{{ $t("diagnosis.needsAi") }}</p>
      <RouterLink :to="{ name: 'settings' }">{{ $t("diagnosis.goToSettings") }}</RouterLink>
    </section>

    <section v-else-if="!finishedScan" class="card">
      <p>{{ $t("files.noScan") }}</p>
      <RouterLink :to="{ name: 'scan' }">{{ $t("files.goToScan") }}</RouterLink>
    </section>

    <template v-else>
      <p v-if="diagnosis.running" class="card running" role="status" data-test="running">
        {{ $t("diagnosis.running", { count: diagnosis.items.length || 40, model }) }}
      </p>

      <section v-if="diagnosis.items.length > 0" class="totals" data-test="totals">
        <div v-for="level in LEVELS" :key="level" class="card total">
          <RiskBadge :level="level" />
          <strong>{{ formatBytes(diagnosis.totals[level].bytes) }}</strong>
          <span class="muted">{{
            $t("diagnosis.items", { count: diagnosis.totals[level].count })
          }}</span>
        </div>
      </section>

      <section class="card">
        <p v-if="diagnosis.error" class="error" role="alert">
          {{ $t("errors.generic", { message: diagnosis.error }) }}
        </p>
        <p
          v-if="!diagnosis.running && diagnosis.pending > 0 && diagnosis.items.length > 0"
          class="muted"
          data-test="pending"
        >
          {{ $t("diagnosis.pending", { count: diagnosis.pending }) }}
        </p>
        <p v-if="diagnosis.fallbacks.length > 0" class="warning-text" data-test="fallbacks">
          {{
            $t("files.batchFallback", {
              count: diagnosis.fallbacks.length,
              message: diagnosis.fallbacks[0]?.analysis?.aiError,
            })
          }}
        </p>
        <p v-if="!diagnosis.loading && diagnosis.items.length === 0" class="muted">
          {{ $t("files.empty") }}
        </p>
        <FileTable
          v-else
          :items="files"
          :max-size="diagnosis.maxSize"
          :home="home"
          :analyses="analysis.byFileId"
          :analyzing="analysis.analyzing"
          :selectable="false"
          show-summary
          @analyze="analysis.open"
          @reveal="scan.revealInFinder"
          @open-folder="openFolder"
        />
      </section>
    </template>

    <AnalysisPanel
      v-if="analysis.selected"
      :item="analysis.selected"
      :analysis="analysis.current"
      :analyzing="analysis.isAnalyzing(analysis.selected.id)"
      :ai-enabled="ai.aiEnabled"
      :home="home"
      :error="analysis.error"
      @close="analysis.close"
      @reanalyze="analysis.analyze(analysis.selected, true)"
      @reveal="scan.revealInFinder"
    />
  </div>
</template>

<style scoped>
.totals {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 0.75rem;
  margin-bottom: 1rem;
}

.total {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  align-items: flex-start;
  margin-bottom: 0;
}

.total strong {
  font-size: 1.15rem;
  font-variant-numeric: tabular-nums;
}

.running {
  color: var(--accent);
}

.warning-text {
  color: var(--warning);
}
</style>
