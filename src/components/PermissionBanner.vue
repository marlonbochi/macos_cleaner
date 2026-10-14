<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import type { PermissionTargetDto } from "@/types/generated/PermissionTargetDto";

const props = defineProps<{ target: PermissionTargetDto | null }>();
defineEmits<{ open: []; recheck: []; reveal: [path: string] }>();

const { t } = useI18n();
const name = computed(() => props.target?.name ?? t("permissions.unknownTarget"));
</script>

<template>
  <div class="banner" role="alert">
    <strong>{{ $t("permissions.title") }}</strong>
    <p>{{ $t("permissions.body") }}</p>
    <p v-if="target?.isDevHost" class="dev-note" data-test="dev-note">
      {{ $t("permissions.devNote", { name }) }}
    </p>
    <ol data-test="steps">
      <li>{{ $t("permissions.step1") }}</li>
      <li>{{ $t("permissions.step2", { name }) }}</li>
      <li>{{ $t(target?.isDevHost ? "permissions.step3Dev" : "permissions.step3", { name }) }}</li>
    </ol>
    <div class="row">
      <button class="primary" @click="$emit('open')">{{ $t("permissions.open") }}</button>
      <button
        v-if="target?.appPath"
        data-test="reveal-target"
        @click="$emit('reveal', target.appPath)"
      >
        {{ $t("permissions.reveal", { name }) }}
      </button>
      <button @click="$emit('recheck')">{{ $t("permissions.recheck") }}</button>
    </div>
  </div>
</template>

<style scoped>
.banner {
  background: var(--warning-bg);
  border: 1px solid var(--warning);
  border-radius: var(--radius);
  padding: 0.75rem 1rem;
  margin-bottom: 1rem;
}

.banner strong {
  color: var(--warning);
}

.banner p {
  margin: 0.25rem 0 0.5rem;
}

.dev-note {
  font-style: italic;
}

ol {
  margin: 0 0 0.75rem;
  padding-left: 1.25rem;
}
</style>
