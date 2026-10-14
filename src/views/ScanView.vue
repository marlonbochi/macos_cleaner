<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useRouter } from "vue-router";
import PermissionBanner from "@/components/PermissionBanner.vue";
import ScanProgress from "@/components/ScanProgress.vue";
import { formatBytes, formatDateTime, shortenPath } from "@/composables/format";
import { pickFolders } from "@/services/dialog";
import { useAiSettingsStore } from "@/stores/aiSettings";
import { useDiagnosisStore } from "@/stores/diagnosis";
import { useScanStore } from "@/stores/scan";

const scan = useScanStore();
const ai = useAiSettingsStore();
const diagnosis = useDiagnosisStore();
const router = useRouter();

const SIZE_OPTIONS = [10, 50, 100, 250, 500, 1000].map((mb) => mb * 1_000_000);

const home = computed(() => scan.defaults?.home ?? null);
const showProgress = computed(() => scan.isRunning || scan.progress !== null);
const canStart = computed(() => !scan.isRunning && scan.roots.length > 0);

onMounted(() => {
  if (!ai.saved) void ai.loadSaved();
  if (!scan.defaults) void scan.init();
});

async function addFolders(): Promise<void> {
  for (const path of await pickFolders()) scan.addRoot(path);
}
</script>

<template>
  <div>
    <header>
      <h1>{{ $t("scan.title") }}</h1>
      <p class="muted">{{ $t("scan.subtitle") }}</p>
    </header>

    <PermissionBanner
      v-if="scan.permissions && !scan.permissions.fullDiskAccess"
      :target="scan.permissions.target"
      @open="scan.openFullDiskAccessSettings"
      @recheck="scan.refreshPermissions"
      @reveal="scan.revealInFinder"
    />

    <section class="card">
      <h2>{{ $t("scan.roots") }}</h2>
      <ul class="roots">
        <li v-for="root in scan.roots" :key="root" class="row">
          <span class="mono">{{ shortenPath(root, home) }}</span>
          <span class="spacer" />
          <button class="link" :disabled="scan.isRunning" @click="scan.removeRoot(root)">
            {{ $t("scan.remove") }}
          </button>
        </li>
        <li v-if="scan.roots.length === 0" class="muted">{{ $t("scan.noRoots") }}</li>
      </ul>
      <div class="row">
        <button v-if="home" :disabled="scan.isRunning" @click="scan.addRoot(home)">
          {{ $t("scan.home") }}
        </button>
        <button :disabled="scan.isRunning" @click="scan.addRoot('/Applications')">
          {{ $t("scan.applications") }}
        </button>
        <button :disabled="scan.isRunning" data-test="whole-disk" @click="scan.addRoot('/')">
          {{ $t("scan.wholeDisk") }}
        </button>
        <button :disabled="scan.isRunning" @click="addFolders">{{ $t("scan.addFolder") }}</button>
      </div>

      <div class="row options">
        <label for="min-size">{{ $t("scan.minSize") }}</label>
        <select id="min-size" v-model.number="scan.minFileSize" :disabled="scan.isRunning">
          <option v-for="size in SIZE_OPTIONS" :key="size" :value="size">
            {{ formatBytes(size) }}
          </option>
        </select>
        <span class="spacer" />
        <button class="primary" :disabled="!canStart" data-test="start" @click="scan.start">
          {{ $t("scan.start") }}
        </button>
      </div>
      <p v-if="scan.error" class="error" role="alert">
        {{ $t("errors.generic", { message: scan.error }) }}
      </p>
    </section>

    <ScanProgress
      v-if="showProgress"
      :progress="scan.progress"
      :running="scan.isRunning"
      @cancel="scan.cancel"
    />

    <section v-if="scan.current && !scan.isRunning" class="card">
      <div class="row">
        <div>
          <strong>{{ $t(`scan.status.${scan.current.status}`) }}</strong>
          <span class="muted">
            · {{ $t("scan.lastScanAt", { date: formatDateTime(scan.current.finishedAt) }) }} ·
            {{ formatBytes(scan.current.bytesSeen) }}
          </span>
          <p v-if="scan.current.incomplete" class="error">{{ $t("scan.incomplete") }}</p>
        </div>
        <span class="spacer" />
        <span
          v-if="diagnosis.runningFor !== null && diagnosis.runningFor === scan.current.id"
          class="muted"
          role="status"
          data-test="diagnosis-running"
        >
          {{ $t("diagnosis.runningShort") }}
        </span>
        <button
          v-if="scan.hasCatalog && ai.aiEnabled"
          data-test="view-diagnosis"
          @click="router.push({ name: 'diagnosis' })"
        >
          {{ $t("diagnosis.view") }}
        </button>
        <button
          v-if="scan.hasCatalog"
          class="primary"
          data-test="view-results"
          @click="router.push({ name: 'files' })"
        >
          {{ $t("scan.viewResults") }}
        </button>
      </div>
    </section>

    <details v-if="scan.inaccessible.length > 0" class="card">
      <summary>{{ $t("scan.inaccessibleList") }} ({{ scan.inaccessible.length }})</summary>
      <ul class="mono inaccessible">
        <li v-for="path in scan.inaccessible" :key="path">{{ shortenPath(path, home) }}</li>
      </ul>
    </details>
  </div>
</template>

<style scoped>
.roots {
  list-style: none;
  padding: 0;
  margin: 0 0 0.75rem;
}

.roots li {
  padding: 0.3rem 0;
  border-bottom: 1px solid var(--border);
}

.options {
  margin-top: 1rem;
}

.inaccessible {
  max-height: 200px;
  overflow: auto;
}
</style>
