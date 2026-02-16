<script setup lang="ts">
const checkerUrl = "https://jsoncompat.com/checker" as const;
</script>

<template>
  <div class="checker-embed-shell">
    <div class="checker-embed-chrome" aria-hidden="true">
      <span class="checker-embed-dot checker-embed-dot-red" />
      <span class="checker-embed-dot checker-embed-dot-yellow" />
      <span class="checker-embed-dot checker-embed-dot-green" />
      <span class="checker-embed-url">{{ checkerUrl }}</span>
    </div>

    <iframe
      class="checker-embed-frame"
      :src="checkerUrl"
      title="jsoncompat.com checker"
      loading="eager"
      referrerpolicy="no-referrer"
    />
  </div>
</template>

<style scoped>
.checker-embed-shell {
  position: absolute;
  inset: 1rem;
  display: grid;
  grid-template-rows: auto 1fr;
  width: auto;
  height: auto;
  overflow: visible;
}

.checker-embed-chrome {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.75rem 1rem;
  border: 1px solid rgba(41, 27, 12, 0.14);
  border-bottom: 1px solid rgba(41, 27, 12, 0.1);
  border-radius: 1.25rem 1.25rem 0 0;
  background: rgba(248, 240, 228, 0.96);
  box-shadow: 0 22px 56px rgba(63, 40, 16, 0.14);
}

.checker-embed-dot {
  width: 0.75rem;
  height: 0.75rem;
  border-radius: 999px;
}

.checker-embed-dot-red {
  background: #f87171;
}

.checker-embed-dot-yellow {
  background: #fbbf24;
}

.checker-embed-dot-green {
  background: #4ade80;
}

.checker-embed-url {
  margin-left: 0.5rem;
  color: var(--deck-muted);
  font: 500 0.9rem "IBM Plex Mono", monospace;
}

.checker-embed-frame {
  width: 100%;
  height: 100%;
  min-height: 0;
  border: 1px solid rgba(41, 27, 12, 0.14);
  border-top: 0;
  background: #fff;
  box-shadow: 0 22px 56px rgba(63, 40, 16, 0.14);
}
</style>
