<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import { formatBytes, shortenPath } from "@/composables/format";
import { useSystemTerms } from "@/composables/platform";
import type { AnalysisDto } from "@/types/generated/AnalysisDto";
import type { FileEntryDto } from "@/types/generated/FileEntryDto";
import RiskBadge from "./RiskBadge.vue";

const props = defineProps<{
  item: FileEntryDto;
  analysis: AnalysisDto | null;
  analyzing: boolean;
  aiEnabled: boolean;
  home: string | null;
  error: string | null;
}>();
defineEmits<{ close: []; reanalyze: []; reveal: [path: string] }>();

const { t } = useI18n();
const terms = useSystemTerms();
const copied = ref<string | null>(null);

const sourceLabel = computed(() => {
  const source = props.analysis?.source;
  if (!source) return "";
  if (source.kind === "ai") {
    return t("analysis.source.ai", { provider: source.provider, model: source.model });
  }
  return t(`analysis.source.${source.kind}`);
});

async function copy(command: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(command);
    copied.value = command;
    setTimeout(() => (copied.value = null), 1500);
  } catch {
    copied.value = null;
  }
}
</script>

<template>
  <div class="backdrop" @click.self="$emit('close')">
    <aside class="panel" role="dialog" aria-modal="true" :aria-label="$t('analysis.title')">
      <header>
        <div class="heading">
          <h2>{{ item.name }}</h2>
          <p class="muted mono path" :title="item.path">{{ shortenPath(item.path, home) }}</p>
          <p class="muted">{{ formatBytes(item.sizeOnDisk) }}</p>
        </div>
        <button class="link" data-test="close" @click="$emit('close')">
          {{ $t("analysis.close") }}
        </button>
      </header>

      <p v-if="analyzing" class="muted loading" data-test="analyzing">
        <span class="spinner" aria-hidden="true" /> {{ $t("analysis.analyzing") }}
      </p>
      <p v-if="error" class="error" role="alert">{{ $t("errors.generic", { message: error }) }}</p>

      <template v-if="analysis && !analyzing">
        <div class="row">
          <RiskBadge :level="analysis.level" />
          <span class="muted source" data-test="source">{{ sourceLabel }}</span>
        </div>
        <p v-if="analysis.aiError" class="warning" data-test="ai-error">
          {{ $t("analysis.aiFailed", { message: analysis.aiError }) }}
        </p>

        <p class="summary">{{ analysis.summary }}</p>
        <p>{{ analysis.explanation }}</p>

        <section>
          <h3>{{ $t("analysis.recommendation") }}</h3>
          <p data-test="recommendation">
            {{ $t(`analysis.recommendations.${analysis.recommendation}`, terms) }}
          </p>
        </section>

        <section v-if="analysis.steps.length > 0">
          <h3>{{ $t("analysis.steps") }}</h3>
          <ol>
            <li v-for="step in analysis.steps" :key="step">{{ step }}</li>
          </ol>
        </section>

        <section v-if="analysis.commands.length > 0" data-test="commands">
          <h3>{{ $t("analysis.commands") }}</h3>
          <p class="muted note">{{ $t("analysis.commandsNote", terms) }}</p>
          <div v-for="cmd in analysis.commands" :key="cmd.command" class="command">
            <div class="row">
              <code class="mono">{{ cmd.command }}</code>
              <span class="spacer" />
              <button class="link" @click="copy(cmd.command)">
                {{ copied === cmd.command ? $t("analysis.copied") : $t("analysis.copy") }}
              </button>
            </div>
            <p class="muted">{{ cmd.description }}</p>
          </div>
        </section>

        <p v-if="!aiEnabled" class="muted hint" data-test="configure-ai">
          {{ $t("analysis.configureAi") }}
          <RouterLink :to="{ name: 'settings' }">{{ $t("nav.settings") }}</RouterLink>
        </p>
      </template>

      <footer class="row">
        <button :disabled="analyzing" data-test="reanalyze" @click="$emit('reanalyze')">
          {{ $t("analysis.again") }}
        </button>
        <button @click="$emit('reveal', item.path)">
          {{ $t("common.revealInFinder", terms) }}
        </button>
      </footer>
    </aside>
  </div>
</template>

<style scoped>
.backdrop {
  position: fixed;
  inset: 0;
  background: rgb(0 0 0 / 35%);
  display: flex;
  justify-content: flex-end;
  z-index: 10;
}

.panel {
  width: min(520px, 100%);
  height: 100%;
  overflow: auto;
  background: var(--surface);
  border-left: 1px solid var(--border);
  padding: 1.25rem 1.5rem;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

header {
  display: flex;
  align-items: flex-start;
  gap: 1rem;
}

.heading {
  flex: 1;
  min-width: 0;
}

.heading h2 {
  overflow-wrap: anywhere;
}

.heading p {
  margin: 0;
}

.path {
  overflow-wrap: anywhere;
}

h3 {
  font-size: 0.9rem;
  margin: 0.75rem 0 0.25rem;
}

.summary {
  font-weight: 600;
  margin-top: 0.5rem;
}

.source {
  font-size: 0.8rem;
}

.warning {
  color: var(--warning);
  background: var(--warning-bg);
  border-radius: 6px;
  padding: 0.4rem 0.6rem;
}

ol {
  margin: 0;
  padding-left: 1.25rem;
}

.command {
  background: var(--surface-muted);
  border-radius: 6px;
  padding: 0.5rem 0.6rem;
  margin-bottom: 0.5rem;
}

.command code {
  overflow-wrap: anywhere;
}

.command p {
  margin: 0.25rem 0 0;
}

.note {
  font-size: 0.8rem;
}

.hint {
  margin-top: 1rem;
}

footer {
  margin-top: auto;
  padding-top: 1rem;
}

.loading {
  display: flex;
  align-items: center;
  gap: 0.5rem;
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
