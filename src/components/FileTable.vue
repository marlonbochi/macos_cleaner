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
    <thead>
      <tr>
        <th>{{ $t("files.name") }}</th>
        <th class="size">{{ $t("files.size") }}</th>
        <th class="num">{{ $t("files.items") }}</th>
        <th>{{ $t("files.modified") }}</th>
        <th />
      </tr>
    </thead>
    <tbody>
      <tr v-for="item in items" :key="item.id" :data-test="`row-${item.id}`">
        <td class="name">
          <span class="icon" :title="$t(`files.kind.${item.kind}`)">{{ ICONS[item.kind] }}</span>
          <div class="labels">
            <span class="title">
              {{ item.name }}
              <span v-if="item.isProtected" class="badge" :title="$t('files.protected')">🔒</span>
            </span>
            <span class="muted mono path" :title="item.path">
              {{ shortenPath(parentPath(item.path), home) }}
            </span>
          </div>
        </td>
        <td class="size">
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
            @click="$emit('openFolder', item.path)"
          >
            {{ $t("files.open") }}
          </button>
          <button class="link" data-test="reveal" @click="$emit('reveal', item.path)">
            {{ $t("common.revealInFinder") }}
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

th:first-child,
td.name {
  width: 45%;
}

.name {
  display: flex;
  gap: 0.6rem;
  align-items: center;
}

.icon {
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

.size {
  width: 20%;
}

.size .value {
  display: block;
  font-variant-numeric: tabular-nums;
  font-weight: 500;
}

.num {
  width: 10%;
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
