<script setup lang="ts">
const NAV = [
  { name: "scan", label: "nav.scan", icon: "🔍" },
  { name: "files", label: "nav.files", icon: "📊" },
  { name: "settings", label: "nav.settings", icon: "⚙️" },
] as const;
</script>

<template>
  <div class="shell">
    <aside class="sidebar">
      <div class="brand">{{ $t("app.name") }}</div>
      <nav>
        <RouterLink v-for="item in NAV" :key="item.name" :to="{ name: item.name }" class="nav-link">
          <span aria-hidden="true">{{ item.icon }}</span>
          {{ $t(item.label) }}
        </RouterLink>
      </nav>
    </aside>
    <main class="content">
      <RouterView />
    </main>
  </div>
</template>

<style scoped>
.shell {
  display: grid;
  grid-template-columns: 190px 1fr;
  height: 100vh;
}

.sidebar {
  background: var(--sidebar);
  border-right: 1px solid var(--border);
  padding: 1rem 0.6rem;
}

.brand {
  font-weight: 700;
  padding: 0 0.6rem 1rem;
}

.nav-link {
  display: flex;
  gap: 0.5rem;
  padding: 0.4rem 0.6rem;
  border-radius: 6px;
  color: var(--text);
  text-decoration: none;
}

.nav-link:hover {
  background: var(--surface-muted);
}

.nav-link.router-link-exact-active {
  background: var(--accent);
  color: var(--accent-contrast);
}

.content {
  overflow: auto;
  padding: 1.5rem 2rem;
}

@media (max-width: 1100px) {
  .content {
    padding: 1.25rem 1rem;
  }
}
</style>
