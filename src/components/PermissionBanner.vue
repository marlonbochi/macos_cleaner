<script setup lang="ts">
defineEmits<{ open: []; recheck: [] }>();
</script>

<template>
  <div class="banner" role="alert">
    <strong>{{ $t("permissions.title") }}</strong>
    <p>{{ $t("permissions.body") }}</p>
    <div class="row">
      <button class="primary" @click="$emit('open')">{{ $t("permissions.open") }}</button>
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
</style>
