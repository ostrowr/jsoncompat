<script setup lang="ts">
const fuzzerUrl = "https://jsoncompat.com/fuzzer" as const;
</script>

<template>
  <div class="fuzzer-embed-shell">
    <div class="fuzzer-embed-chrome" aria-hidden="true">
      <span class="fuzzer-embed-dot fuzzer-embed-dot-red" />
      <span class="fuzzer-embed-dot fuzzer-embed-dot-yellow" />
      <span class="fuzzer-embed-dot fuzzer-embed-dot-green" />
      <span class="fuzzer-embed-url">{{ fuzzerUrl }}</span>
    </div>

    <iframe
      class="fuzzer-embed-frame"
      :src="fuzzerUrl"
      title="jsoncompat.com fuzzer"
      loading="eager"
      referrerpolicy="no-referrer"
    />
  </div>
</template>

<style scoped>
.fuzzer-embed-shell {
  position: absolute;
  inset: 1rem;
  display: grid;
  grid-template-rows: auto 1fr;
  width: auto;
  height: auto;
  overflow: visible;
}

.fuzzer-embed-chrome {
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

.fuzzer-embed-dot {
  width: 0.75rem;
  height: 0.75rem;
  border-radius: 999px;
}

.fuzzer-embed-dot-red {
  background: #f87171;
}

.fuzzer-embed-dot-yellow {
  background: #fbbf24;
}

.fuzzer-embed-dot-green {
  background: #4ade80;
}

.fuzzer-embed-url {
  margin-left: 0.5rem;
  color: var(--deck-muted);
  font: 500 0.9rem "IBM Plex Mono", monospace;
}

.fuzzer-embed-frame {
  width: 100%;
  height: 100%;
  min-height: 0;
  border: 1px solid rgba(41, 27, 12, 0.14);
  border-top: 0;
  background: #fff;
  box-shadow: 0 22px 56px rgba(63, 40, 16, 0.14);
}
</style>
