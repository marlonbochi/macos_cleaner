<script setup lang="ts">
import {
  formatBytes,
  formatCount,
  formatDate,
  parentPath,
  shortenPath,
} from "@/composables/format";
import type { FileEntryDto } from "@/types/generated/FileEntryDto";
import SizeBar from "./SizeBar.vue";

defineProps<{ items: FileEntryDto[]; maxSize: number; home: string | null }>();
defineEmits<{ openFolder: [path: string]; reveal: [path: string] }>();

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
      <col />
      <col class="col-size" />
      <col class="col-num" />
      <col class="col-date" />
      <col class="col-actions" />
    </colgroup>
    <thead>
      <tr>
        <th>{{ $t("files.name") }}</th>
        <th>{{ $t("files.size") }}</th>
        <th class="num">{{ $t("files.items") }}</th>
        <th>{{ $t("files.modified") }}</th>
        <th />
      </tr>
    </thead>
    <tbody>
      <tr v-for="item in items" :key="item.id" :data-test="`row-${item.id}`">
        <td>
          <div class="name">
            <span class="icon" :title="$t(`files.kind.${item.kind}`)">{{ ICONS[item.kind] }}</span>
            <div class="labels">
              <span class="title" :title="item.name">
                {{ item.name }}
                <span v-if="item.isProtected" class="badge" :title="$t('files.protected')">🔒</span>
              </span>
              <span class="muted mono path" :title="item.path">
                {{ shortenPath(parentPath(item.path), home) }}
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
            :title="$t('common.revealInFinder')"
            @click="$emit('reveal', item.path)"
          >
            {{ $t("files.reveal") }}
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
  width: 105px;
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
.path {
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
</style>
