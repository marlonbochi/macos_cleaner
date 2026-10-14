<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import AnalysisPanel from "@/components/AnalysisPanel.vue";
import FileTable from "@/components/FileTable.vue";
import PaginationBar from "@/components/PaginationBar.vue";
import { formatBytes, formatDateTime, parentPath, shortenPath } from "@/composables/format";
import { useAiSettingsStore } from "@/stores/aiSettings";
import { useAnalysisStore } from "@/stores/analysis";
import { useCatalogStore } from "@/stores/catalog";
import { useScanStore } from "@/stores/scan";
import type { FileKindDto } from "@/types/generated/FileKindDto";

const scan = useScanStore();
const catalog = useCatalogStore();
const analysis = useAnalysisStore();
const ai = useAiSettingsStore();

const home = computed(() => scan.defaults?.home ?? null);
const finishedScan = computed(() => (scan.hasCatalog ? scan.current : null));

type KindChoice = "all" | "files" | "folders";
const kindChoice = ref<KindChoice>("all");
const extensionText = ref("");
const KIND_FILTERS: Record<KindChoice, FileKindDto[]> = {
  all: [],
  files: ["File"],
  folders: ["Directory", "Package"],
};

onMounted(async () => {
  if (!ai.saved) void ai.loadSaved();
  if (!scan.defaults) await scan.init();
});

// Show risk badges for rows that were analysed before.
watch(
  () => catalog.items,
  (items) => void analysis.loadFor(items.map((i) => i.id)),
);

watch(
  () => finishedScan.value?.id,
  (id) => {
    if (id !== undefined) void catalog.open(id);
  },
  { immediate: true },
);

watch(kindChoice, (choice) => void catalog.setFilter({ kinds: KIND_FILTERS[choice] }));

let extensionTimer: ReturnType<typeof setTimeout> | undefined;
watch(extensionText, (text) => {
  clearTimeout(extensionTimer);
  extensionTimer = setTimeout(() => {
    const extensions = text
      .split(/[\s,]+/)
      .map((e) => e.replace(/^\./, "").toLowerCase())
      .filter(Boolean);
    void catalog.setFilter({ extensions });
  }, 300);
});

const breadcrumb = computed(() =>
  catalog.folder ? shortenPath(catalog.folder, home.value) : null,
);

const batchRunning = computed(() => catalog.selected.some((id) => analysis.analyzing.includes(id)));

async function analyzeSelected(): Promise<void> {
  await analysis.analyzeMany([...catalog.selected]);
  if (!analysis.error) catalog.clearSelection();
}

function goUp(): void {
  if (!catalog.folder) return;
  const isRoot = finishedScan.value?.roots.includes(catalog.folder) ?? false;
  void catalog.openFolder(isRoot ? null : parentPath(catalog.folder));
}
</script>

<template>
  <div>
    <header class="row">
      <div>
        <h1>{{ $t("files.title") }}</h1>
        <p v-if="finishedScan" class="muted">
          {{ $t("scan.lastScanAt", { date: formatDateTime(finishedScan.finishedAt) }) }}
          · {{ formatBytes(finishedScan.bytesSeen) }}
          <span v-if="finishedScan.incomplete" class="error"> · {{ $t("scan.incomplete") }}</span>
        </p>
      </div>
    </header>

    <section v-if="!finishedScan" class="card">
      <p>{{ $t("files.noScan") }}</p>
      <RouterLink :to="{ name: 'scan' }">{{ $t("files.goToScan") }}</RouterLink>
    </section>

    <template v-else>
      <section class="card filters row">
        <label for="kind">{{ $t("files.filters.kind") }}</label>
        <select id="kind" v-model="kindChoice" :disabled="catalog.folder !== null">
          <option value="all">{{ $t("files.filters.all") }}</option>
          <option value="files">{{ $t("files.filters.filesOnly") }}</option>
          <option value="folders">{{ $t("files.filters.foldersOnly") }}</option>
        </select>
        <label for="ext">{{ $t("files.filters.extension") }}</label>
        <input
          id="ext"
          v-model="extensionText"
          :placeholder="$t('files.filters.extensionPlaceholder')"
          :disabled="catalog.folder !== null"
        />
      </section>

      <section class="card">
        <nav v-if="breadcrumb" class="row breadcrumb">
          <button class="link" data-test="up" @click="goUp">← {{ $t("files.up") }}</button>
          <button class="link" @click="catalog.openFolder(null)">{{ $t("files.root") }}</button>
          <span class="mono">/ {{ breadcrumb }}</span>
        </nav>
        <div
          v-if="ai.aiEnabled && catalog.selected.length > 0"
          class="row selection"
          data-test="selection-bar"
        >
          <strong>{{ $t("files.selection", { count: catalog.selected.length }) }}</strong>
          <button
            class="primary"
            data-test="analyze-selected"
            :disabled="batchRunning"
            :title="$t('files.batchHint')"
            @click="analyzeSelected"
          >
            {{
              batchRunning
                ? $t("files.analyzingSelected", { count: catalog.selected.length })
                : $t("files.analyzeSelected")
            }}
          </button>
          <button class="link" :disabled="batchRunning" @click="catalog.clearSelection">
            {{ $t("files.clearSelection") }}
          </button>
        </div>
        <p v-if="analysis.lastBatch" class="muted" data-test="batch-result">
          {{ $t("files.batchDone", { count: analysis.lastBatch.count }) }}
          <span v-if="analysis.lastBatch.fallbacks > 0" class="warning-text">
            {{
              $t("files.batchFallback", {
                count: analysis.lastBatch.fallbacks,
                message: analysis.lastBatch.aiError,
              })
            }}
          </span>
        </p>
        <p v-if="analysis.error && !analysis.selected" class="error" role="alert">
          {{ $t("errors.generic", { message: analysis.error }) }}
        </p>
        <p v-if="catalog.error" class="error" role="alert">
          {{ $t("errors.generic", { message: catalog.error }) }}
        </p>
        <p v-if="!catalog.loading && catalog.items.length === 0" class="muted">
          {{ $t("files.empty") }}
        </p>
        <FileTable
          v-else
          :items="catalog.items"
          :max-size="catalog.maxSize"
          :home="home"
          :analyses="analysis.byFileId"
          :analyzing="analysis.analyzing"
          :selected="catalog.selected"
          :selectable="ai.aiEnabled"
          @toggle="catalog.toggleSelected"
          @toggle-all="catalog.setSelected"
          @open-folder="catalog.openFolder"
          @reveal="catalog.revealInFinder"
          @analyze="analysis.open"
        />
        <PaginationBar
          :offset="catalog.offset"
          :count="catalog.items.length"
          :total="catalog.total"
          :has-previous="catalog.hasPrevious"
          :has-next="catalog.hasNext"
          @previous="catalog.previousPage"
          @next="catalog.nextPage"
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
      @reveal="catalog.revealInFinder"
    />
  </div>
</template>

<style scoped>
.filters {
  gap: 0.75rem;
}

.selection {
  margin-bottom: 0.75rem;
  padding: 0.5rem 0.75rem;
  border-radius: 6px;
  background: color-mix(in srgb, var(--accent) 12%, transparent);
}

.warning-text {
  color: var(--warning);
}

.breadcrumb {
  margin-bottom: 0.5rem;
}
</style>
