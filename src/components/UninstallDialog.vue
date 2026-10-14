<script setup lang="ts">
import { computed, nextTick, onMounted, ref } from "vue";
import { formatBytes, shortenPath } from "@/composables/format";
import type { AppFootprintDto } from "@/types/generated/AppFootprintDto";
import type { UninstallReportDto } from "@/types/generated/UninstallReportDto";

const props = defineProps<{
  footprint: AppFootprintDto | null;
  loading: boolean;
  running: boolean;
  report: UninstallReportDto | null;
  error: string | null;
  home: string | null;
}>();
const emit = defineEmits<{ confirm: [typed: string]; close: [] }>();

const typed = ref("");
const input = ref<HTMLInputElement | null>(null);

const total = computed(() => props.footprint?.items.reduce((sum, i) => sum + i.size, 0) ?? 0);
/** Same rule as the backend: the exact phrase, surrounding spaces ignored. */
const confirmed = computed(
  () => !!props.footprint && typed.value.trim() === props.footprint.confirmationPhrase,
);
const done = computed(() => props.report !== null);

onMounted(async () => {
  await nextTick();
  input.value?.focus();
});

function close(): void {
  if (!props.running && !props.report?.quitting) emit("close");
}

function submit(): void {
  if (confirmed.value && !props.running) emit("confirm", typed.value);
}
</script>

<template>
  <div class="backdrop" @click.self="close" @keydown.esc="close">
    <div
      class="dialog"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="uninstall-title"
      data-test="uninstall-dialog"
    >
      <h2 id="uninstall-title">{{ $t("uninstall.dialogTitle") }}</h2>

      <template v-if="report">
        <p data-test="uninstall-result">
          {{ $t("uninstall.result", { count: report.trashed.length, keys: report.keysRemoved }) }}
        </p>
        <ul v-if="report.failures.length > 0" class="failures" data-test="uninstall-failures">
          <li v-for="f in report.failures" :key="f.target" class="error">
            <span class="mono">{{ shortenPath(f.target, home) }}</span> — {{ f.reason }}
          </li>
        </ul>
        <p v-if="report.quitting" class="muted">{{ $t("uninstall.quitting") }}</p>
      </template>

      <template v-else>
        <p class="warning">{{ $t("uninstall.warning") }}</p>
        <p v-if="loading" class="muted">{{ $t("common.loading") }}</p>
        <template v-else-if="footprint">
          <ul class="items" data-test="uninstall-items">
            <li v-for="item in footprint.items" :key="item.path">
              <span>{{ $t(`uninstall.kind.${item.kind}`) }}</span>
              <span class="mono path" :title="item.path">{{ shortenPath(item.path, home) }}</span>
              <span class="size">{{ formatBytes(item.size) }}</span>
            </li>
            <li v-if="footprint.storedKeys.length > 0" data-test="uninstall-keys">
              <span>{{ $t("uninstall.keys") }}</span>
              <span class="mono path">{{ footprint.storedKeys.join(", ") }}</span>
              <span class="size" />
            </li>
          </ul>
          <p class="muted">{{ $t("uninstall.total", { size: formatBytes(total) }) }}</p>
          <p v-if="!footprint.bundleFound" class="muted" data-test="uninstall-dev">
            {{ $t("uninstall.noBundle") }}
          </p>
        </template>

        <form @submit.prevent="submit">
          <label for="uninstall-confirm">
            {{ $t("uninstall.typeToConfirm", { phrase: footprint?.confirmationPhrase ?? "" }) }}
          </label>
          <input
            id="uninstall-confirm"
            ref="input"
            v-model="typed"
            autocomplete="off"
            spellcheck="false"
            :placeholder="footprint?.confirmationPhrase ?? ''"
            :disabled="running || !footprint"
          />
          <p v-if="error" class="error" role="alert">
            {{ $t("errors.generic", { message: error }) }}
          </p>
          <div class="row actions">
            <span class="spacer" />
            <button type="button" data-test="uninstall-cancel" :disabled="running" @click="close">
              {{ $t("common.cancel") }}
            </button>
            <button
              type="submit"
              class="danger"
              data-test="uninstall-confirm"
              :disabled="!confirmed || running"
            >
              {{ running ? $t("uninstall.running") : $t("uninstall.confirm") }}
            </button>
          </div>
        </form>
      </template>

      <div v-if="done && !report?.quitting" class="row actions">
        <span class="spacer" />
        <button type="button" @click="emit('close')">{{ $t("analysis.close") }}</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.backdrop {
  position: fixed;
  inset: 0;
  background: rgb(0 0 0 / 40%);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 20;
  padding: 1rem;
}

.dialog {
  background: var(--surface);
  border-radius: var(--radius);
  box-shadow: 0 10px 40px rgb(0 0 0 / 25%);
  width: min(560px, 100%);
  max-height: 90vh;
  overflow: auto;
  padding: 1.25rem 1.5rem;
}

.warning {
  color: var(--danger);
}

.items {
  list-style: none;
  padding: 0;
  margin: 0.75rem 0;
  border: 1px solid var(--border);
  border-radius: 6px;
}

.items li {
  display: grid;
  grid-template-columns: 9rem minmax(0, 1fr) 5.5rem;
  gap: 0.5rem;
  padding: 0.4rem 0.6rem;
  border-bottom: 1px solid var(--border);
  font-size: 0.85rem;
}

.items li:last-child {
  border-bottom: none;
}

.path {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text-muted);
}

.size {
  text-align: right;
  font-variant-numeric: tabular-nums;
}

form {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  margin-top: 0.75rem;
}

.actions {
  margin-top: 0.75rem;
  gap: 0.5rem;
}

.failures {
  padding-left: 1rem;
}
</style>
