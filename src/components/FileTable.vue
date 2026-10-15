<script setup lang="ts">
import { computed } from "vue";
import {
  formatBytes,
  formatCount,
  formatDate,
  parentPath,
  shortenPath,
} from "@/composables/format";
import { useSystemTerms } from "@/composables/platform";
import type { AnalysisDto } from "@/types/generated/AnalysisDto";
import type { FileEntryDto } from "@/types/generated/FileEntryDto";
import RiskBadge from "./RiskBadge.vue";
import SizeBar from "./SizeBar.vue";

const props = withDefaults(
  defineProps<{
    items: FileEntryDto[];
    maxSize: number;
    home: string | null;
    analyses?: Record<number, AnalysisDto>;
    analyzing?: number[];
    /** Selected ids; protected items cannot be selected. */
    selected?: number[];
    /** Show the selection column (only useful when batch AI analysis is available). */
    selectable?: boolean;
    /** Show each item's analysis summary under its location. */
    showSummary?: boolean;
  }>(),
  {
    analyses: () => ({}),
    analyzing: () => [],
    selected: () => [],
    selectable: true,
    showSummary: false,
  },
);
defineEmits<{
  openFolder: [path: string];
  reveal: [path: string];
  analyze: [item: FileEntryDto];
  toggle: [id: number];
  toggleAll: [ids: number[], value: boolean];
}>();

const terms = useSystemTerms();
const selectableIds = computed(() => props.items.filter((i) => !i.isProtected).map((i) => i.id));
const selectedOnPage = computed(
  () => selectableIds.value.filter((id) => props.selected.includes(id)).length,
);
const allSelected = computed(
  () => selectableIds.value.length > 0 && selectedOnPage.value === selectableIds.value.length,
);
const someSelected = computed(() => selectedOnPage.value > 0 && !allSelected.value);

const ICONS: Record<FileEntryDto["kind"], string> = {
  File: "📄",
  Directory: "📁",
  Package: "📦",
};
</script>

<template>
  <table class="file-table">
    <!-- Fixed widths for data columns; the name column takes all remaining space. -->
    <colgroup>
      <col v-if="selectable" class="col-check" />
      <col />
      <col class="col-size" />
      <col class="col-num" />
      <col class="col-date" />
      <col class="col-actions" />
    </colgroup>
    <thead>
      <tr>
        <th v-if="selectable" class="check">
          <input
            type="checkbox"
            data-test="select-page"
            :aria-label="$t('files.selectPage')"
            :checked="allSelected"
            :indeterminate="someSelected"
            :disabled="selectableIds.length === 0"
            @change="$emit('toggleAll', selectableIds, !allSelected)"
          />
        </th>
        <th>{{ $t("files.name") }}</th>
        <th>{{ $t("files.size") }}</th>
        <th class="num">{{ $t("files.items") }}</th>
        <th>{{ $t("files.modified") }}</th>
        <th />
      </tr>
    </thead>
    <tbody>
      <tr
        v-for="item in items"
        :key="item.id"
        :data-test="`row-${item.id}`"
        :class="{ selected: selected.includes(item.id) }"
      >
        <td v-if="selectable" class="check">
          <input
            type="checkbox"
            data-test="select"
            :aria-label="$t('files.select', { name: item.name })"
            :checked="selected.includes(item.id)"
            :disabled="item.isProtected"
            @change="$emit('toggle', item.id)"
          />
        </td>
        <td>
          <div class="name">
            <span class="icon" :title="$t(`files.kind.${item.kind}`)">{{ ICONS[item.kind] }}</span>
            <div class="labels">
              <span class="title" :title="item.name">
                {{ item.name }}
                <span v-if="item.isProtected" class="badge" :title="$t('files.protected')">🔒</span>
                <RiskBadge
                  v-if="analyses[item.id]"
                  :level="analyses[item.id]!.level"
                  :title="analyses[item.id]!.summary"
                  class="row-risk"
                />
              </span>
              <span class="muted mono path" :title="item.path">
                {{ shortenPath(parentPath(item.path), home) }}
              </span>
              <span
                v-if="showSummary && analyses[item.id]"
                class="summary"
                data-test="summary"
                :title="analyses[item.id]!.summary"
              >
                {{ analyses[item.id]!.summary }}
              </span>
            </div>
          </div>
        </td>
        <td>
          <span class="value">{{ formatBytes(item.sizeOnDisk) }}</span>
          <SizeBar :value="item.sizeOnDisk" :max="maxSize" />
        </td>
        <td class="num">{{ item.kind === "File" ? "" : formatCount(item.fileCount) }}</td>
        <td>{{ formatDate(item.modifiedAt) }}</td>
        <td class="actions">
          <button
            class="link"
            data-test="analyze"
            :disabled="analyzing.includes(item.id)"
            :title="$t('files.analyzeTitle')"
            @click="$emit('analyze', item)"
          >
            {{ analyzing.includes(item.id) ? "…" : $t("files.analyze") }}
          </button>
          <button
            v-if="item.kind === 'Directory'"
            class="link"
            data-test="open-folder"
            :title="$t('files.openTitle')"
            @click="$emit('openFolder', item.path)"
          >
            {{ $t("files.open") }}
          </button>
          <button
            class="link"
            data-test="reveal"
            :title="$t('common.revealInFinder', terms)"
            @click="$emit('reveal', item.path)"
          >
            {{ terms.fileManagerButton }}
          </button>
        </td>
      </tr>
    </tbody>
  </table>
</template>

<style scoped>
.file-table {
  width: 100%;
  border-collapse: collapse;
  table-layout: fixed;
}

.col-check {
  width: 32px;
}

.check {
  text-align: center;
  padding-left: 0.25rem;
  padding-right: 0.25rem;
}

tr.selected {
  background: color-mix(in srgb, var(--accent) 10%, transparent);
}

.col-size {
  width: 140px;
}

.col-num {
  width: 80px;
}

.col-date {
  width: 90px;
}

.col-actions {
  width: 170px;
}

th {
  text-align: left;
  font-weight: 500;
  color: var(--text-muted);
  font-size: 0.8rem;
  padding: 0.4rem 0.5rem;
  border-bottom: 1px solid var(--border);
}

td {
  padding: 0.45rem 0.5rem;
  border-bottom: 1px solid var(--border);
  vertical-align: middle;
}

tbody tr:hover {
  background: var(--surface-muted);
}

.name {
  display: flex;
  gap: 0.6rem;
  align-items: center;
  min-width: 0;
}

.icon {
  flex: none;
  font-size: 1.2rem;
}

.labels {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.title,
.path,
.summary {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.value {
  display: block;
  font-variant-numeric: tabular-nums;
  font-weight: 500;
}

.num {
  text-align: right;
  font-variant-numeric: tabular-nums;
}

.actions {
  text-align: right;
  white-space: nowrap;
}

.badge {
  font-size: 0.8rem;
}

.summary {
  font-size: 0.85rem;
}

.row-risk {
  margin-left: 0.35rem;
}
</style>
