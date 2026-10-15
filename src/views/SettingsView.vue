<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import UninstallDialog from "@/components/UninstallDialog.vue";
import { useSystemTerms } from "@/composables/platform";
import { LOCALE_NAMES, SUPPORTED_LOCALES, isAppLocale } from "@/i18n";
import { useAiSettingsStore } from "@/stores/aiSettings";
import { usePreferencesStore } from "@/stores/preferences";
import { useScanStore } from "@/stores/scan";
import { useUninstallStore } from "@/stores/uninstall";
import type { AiProviderIdDto } from "@/types/generated/AiProviderIdDto";

const ai = useAiSettingsStore();
const preferences = usePreferencesStore();
const scan = useScanStore();
const uninstall = useUninstallStore();
const uninstallOpen = ref(false);
const terms = useSystemTerms();

onMounted(() => void ai.load());

const isOllama = computed(() => ai.form.provider === "ollama");
const showUrl = computed(() => isOllama.value || ai.form.customUrl);

function onLocaleChange(event: Event): void {
  const value = (event.target as HTMLSelectElement).value;
  if (isAppLocale(value)) void preferences.setLocale(value);
}

function onProviderChange(event: Event): void {
  const value = (event.target as HTMLSelectElement).value;
  void ai.selectProvider(value === "" ? null : (value as AiProviderIdDto));
}

function onUrlInput(event: Event): void {
  ai.setBaseUrl((event.target as HTMLInputElement).value);
}

function onCustomUrl(event: Event): void {
  ai.setCustomUrl((event.target as HTMLInputElement).checked);
}

function openUninstall(): void {
  uninstall.reset();
  uninstallOpen.value = true;
  if (!scan.defaults) void scan.init();
  void uninstall.load();
}

function closeUninstall(): void {
  uninstallOpen.value = false;
  uninstall.reset();
}
</script>

<template>
  <div>
    <header>
      <h1>{{ $t("settings.title") }}</h1>
    </header>

    <section class="card">
      <h2>{{ $t("settings.languageTitle") }}</h2>
      <div class="field">
        <select
          id="locale"
          :aria-label="$t('settings.languageTitle')"
          :value="preferences.locale"
          @change="onLocaleChange"
        >
          <option v-for="code in SUPPORTED_LOCALES" :key="code" :value="code">
            {{ LOCALE_NAMES[code] }}
          </option>
        </select>
        <small class="muted">{{ $t("settings.languageHelp") }}</small>
        <small v-if="preferences.error" class="error" role="alert">
          {{ $t("errors.generic", { message: preferences.error }) }}
        </small>
      </div>
    </section>

    <section class="card">
      <h2>{{ $t("settings.aiTitle") }}</h2>
      <p class="muted">{{ $t("settings.aiSubtitle") }}</p>

      <div class="field">
        <label for="provider">{{ $t("settings.provider") }}</label>
        <select id="provider" :value="ai.form.provider ?? ''" @change="onProviderChange">
          <option value="">{{ $t("settings.none") }}</option>
          <option v-for="p in ai.saved?.providers ?? []" :key="p.id" :value="p.id">
            {{ p.label }}
          </option>
        </select>
      </div>

      <template v-if="ai.providerInfo">
        <div v-if="ai.providerInfo.needsApiKey" class="field">
          <label for="api-key">{{ $t("settings.apiKey") }}</label>
          <div class="row">
            <input
              id="api-key"
              v-model="ai.form.apiKey"
              type="password"
              autocomplete="off"
              spellcheck="false"
              :placeholder="
                ai.providerInfo.hasApiKey
                  ? $t('settings.apiKeyPlaceholderSaved', terms)
                  : $t('settings.apiKeyPlaceholder')
              "
            />
            <button
              v-if="ai.providerInfo.hasApiKey"
              class="link"
              data-test="remove-key"
              @click="ai.save(true)"
            >
              {{ $t("settings.removeKey") }}
            </button>
          </div>
          <small class="muted">{{ $t("settings.apiKeyHelp", terms) }}</small>
        </div>

        <div class="field">
          <label for="base-url">{{ $t("settings.baseUrl") }}</label>
          <label v-if="!isOllama" class="check">
            <input
              type="checkbox"
              data-test="custom-url"
              :checked="ai.form.customUrl"
              @change="onCustomUrl"
            />
            {{ $t("settings.customUrl") }}
          </label>
          <input
            v-if="showUrl"
            id="base-url"
            :value="ai.form.baseUrl"
            spellcheck="false"
            :placeholder="ai.providerInfo.defaultBaseUrl"
            @input="onUrlInput"
          />
          <small class="muted">
            {{
              isOllama
                ? $t("settings.baseUrlHelp", { url: ai.providerInfo.defaultBaseUrl })
                : !ai.form.customUrl
                  ? $t("settings.officialUrl", { url: ai.providerInfo.defaultBaseUrl })
                  : ""
            }}
          </small>
        </div>

        <div class="field">
          <label for="model">{{ $t("settings.model") }}</label>
          <div class="row">
            <select
              id="model"
              v-model="ai.form.model"
              :disabled="!ai.modelsCurrent || ai.modelsLoading || ai.models.length === 0"
            >
              <option value="" disabled>{{ $t("settings.modelPlaceholder") }}</option>
              <option v-for="m in ai.models" :key="m.id" :value="m.id">
                {{ m.label === m.id ? m.id : `${m.label} — ${m.id}` }}
                {{
                  m.id === ai.providerInfo.defaultModel ? `(${$t("settings.modelDefault")})` : ""
                }}
              </option>
            </select>
            <button
              :disabled="ai.modelsLoading || !ai.hasKey"
              data-test="load-models"
              @click="ai.fetchModels"
            >
              {{ ai.modelsCurrent ? $t("settings.reloadModels") : $t("settings.loadModels") }}
            </button>
          </div>
          <small v-if="ai.modelsLoading" class="muted">{{ $t("settings.modelsLoading") }}</small>
          <small v-else-if="ai.modelsError" class="error" data-test="models-error">
            {{ $t("settings.modelsError", { message: ai.modelsError }) }}
          </small>
          <small v-else-if="!ai.hasKey" class="muted" data-test="models-need-key">
            {{ $t("settings.modelsNeedKey") }}
          </small>
          <small v-else-if="!ai.modelsCurrent" class="muted">
            {{ $t("settings.modelsNeedLoad") }}
          </small>
          <small v-else-if="ai.models.length === 0" class="muted" data-test="models-empty">
            {{ isOllama ? $t("settings.modelsEmptyOllama", terms) : $t("settings.modelsEmpty") }}
          </small>
          <small v-if="ai.savedModelMissing" class="warning-text" data-test="saved-model-missing">
            {{ $t("settings.savedModelMissing", { model: ai.saved?.model }) }}
          </small>
        </div>
      </template>

      <div class="row actions">
        <button
          class="primary"
          :disabled="ai.saving || !ai.canSave"
          data-test="save"
          @click="ai.save()"
        >
          {{ $t("settings.save") }}
        </button>
        <button
          v-if="ai.form.provider"
          :disabled="ai.testing || ai.saving || !ai.canSave"
          data-test="test"
          @click="ai.test"
        >
          {{ ai.testing ? $t("settings.testing") : $t("settings.test") }}
        </button>
      </div>

      <p v-if="ai.message === 'connected' && ai.testResult" class="success" data-test="message">
        {{ $t("settings.connected", ai.testResult) }}
      </p>
      <p v-else-if="ai.message" class="success" data-test="message">
        {{ $t(`settings.${ai.message}`, terms) }}
      </p>
      <p v-if="ai.error" class="error" role="alert">
        {{ $t("errors.generic", { message: ai.error }) }}
      </p>
    </section>

    <section class="card">
      <h2>{{ $t("settings.privacyTitle") }}</h2>
      <p class="muted">{{ $t("settings.privacy") }}</p>
    </section>

    <section class="card danger-zone">
      <h2>{{ $t("uninstall.title") }}</h2>
      <p class="muted">{{ $t("uninstall.description", terms) }}</p>
      <button class="danger" data-test="open-uninstall" @click="openUninstall">
        {{ $t("uninstall.open") }}
      </button>
    </section>

    <UninstallDialog
      v-if="uninstallOpen"
      :footprint="uninstall.footprint"
      :loading="uninstall.loading"
      :running="uninstall.running"
      :report="uninstall.report"
      :error="uninstall.error"
      :home="scan.defaults?.home ?? null"
      @confirm="uninstall.uninstall"
      @close="closeUninstall"
    />
  </div>
</template>

<style scoped>
.danger-zone {
  border-color: color-mix(in srgb, var(--danger) 45%, var(--border));
}

.field {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  margin: 0.9rem 0;
  max-width: 620px;
}

.field .row input,
.field .row select {
  flex: 1;
  min-width: 0;
}

label {
  font-weight: 500;
}

label.check {
  font-weight: 400;
  display: flex;
  align-items: center;
  gap: 0.4rem;
}

.actions {
  margin-top: 1rem;
}

.success {
  color: var(--success);
  margin-top: 0.75rem;
}

.warning-text {
  color: var(--warning);
}
</style>
