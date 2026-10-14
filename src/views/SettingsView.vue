<script setup lang="ts">
import { onMounted } from "vue";
import { useAiSettingsStore } from "@/stores/aiSettings";
import type { AiProviderIdDto } from "@/types/generated/AiProviderIdDto";

const ai = useAiSettingsStore();

onMounted(() => void ai.load());

function onProviderChange(event: Event): void {
  const value = (event.target as HTMLSelectElement).value;
  ai.selectProvider(value === "" ? null : (value as AiProviderIdDto));
}
</script>

<template>
  <div>
    <header>
      <h1>{{ $t("settings.title") }}</h1>
    </header>

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
                  ? $t('settings.apiKeyPlaceholderSaved')
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
          <small class="muted">{{ $t("settings.apiKeyHelp") }}</small>
        </div>

        <div class="field">
          <label for="model">{{ $t("settings.model") }}</label>
          <input
            id="model"
            v-model="ai.form.model"
            spellcheck="false"
            :placeholder="ai.providerInfo.defaultModel"
          />
          <small class="muted">
            {{ $t("settings.modelHelp", { model: ai.providerInfo.defaultModel }) }}
          </small>
        </div>

        <div class="field">
          <label for="base-url">{{ $t("settings.baseUrl") }}</label>
          <input
            id="base-url"
            v-model="ai.form.baseUrl"
            spellcheck="false"
            :placeholder="ai.providerInfo.defaultBaseUrl"
          />
          <small class="muted">
            {{ $t("settings.baseUrlHelp", { url: ai.providerInfo.defaultBaseUrl }) }}
          </small>
        </div>
      </template>

      <div class="row actions">
        <button class="primary" :disabled="ai.saving" data-test="save" @click="ai.save()">
          {{ $t("settings.save") }}
        </button>
        <button
          v-if="ai.form.provider"
          :disabled="ai.testing || ai.saving"
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
        {{ $t(`settings.${ai.message}`) }}
      </p>
      <p v-if="ai.error" class="error" role="alert">
        {{ $t("errors.generic", { message: ai.error }) }}
      </p>
    </section>

    <section class="card">
      <h2>{{ $t("settings.privacyTitle") }}</h2>
      <p class="muted">{{ $t("settings.privacy") }}</p>
    </section>
  </div>
</template>

<style scoped>
.field {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  margin: 0.9rem 0;
  max-width: 560px;
}

.field .row input {
  flex: 1;
}

label {
  font-weight: 500;
}

.actions {
  margin-top: 1rem;
}

.success {
  color: var(--success);
  margin-top: 0.75rem;
}
</style>
