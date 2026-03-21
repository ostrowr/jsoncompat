<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";

interface FuzzCase {
  seed: string;
  candidate: string;
  verdict: "searching" | "interesting" | "breaking";
  note: string;
}

const cases: readonly FuzzCase[] = [
  {
    seed: "seed 1042",
    candidate: '{ "retries": 4, "mode": "fast" }',
    verdict: "searching",
    note: "valid under both versions",
  },
  {
    seed: "seed 1079",
    candidate: '{ "retries": 5, "mode": "fast" }',
    verdict: "interesting",
    note: "boundary value at the edge of the contract",
  },
  {
    seed: "seed 1081",
    candidate: '{ "retries": 5, "mode": "fast" }',
    verdict: "breaking",
    note: "old reader accepted it, new reader rejects it",
  },
  {
    seed: "seed 1096",
    candidate: '{ "retries": -1, "mode": "fast" }',
    verdict: "searching",
    note: "filtered out by primitive constraints",
  },
] as const;

const index = ref(0);
let intervalId: number | null = null;

const activeCase = computed(() => cases[index.value] ?? cases[0]);
const statusText = computed(() => {
  switch (activeCase.value.verdict) {
    case "breaking":
      return "counterexample found";
    case "interesting":
      return "shrinking toward boundary";
    case "searching":
      return "mutating candidates";
  }
});

const statusClass = computed(() => `fuzz-status fuzz-status-${activeCase.value.verdict}`);

onMounted(() => {
  intervalId = window.setInterval(() => {
    index.value = (index.value + 1) % cases.length;
  }, 1800);
});

onBeforeUnmount(() => {
  if (intervalId !== null) {
    window.clearInterval(intervalId);
  }
});
</script>

<template>
  <div class="fuzz-demo" aria-live="polite">
    <div class="fuzz-topline">
      <div class="deck-kicker">Fuzzing where static analysis runs out of road</div>
      <div :class="statusClass">{{ statusText }}</div>
    </div>

    <div class="fuzz-grid">
      <div class="fuzz-card">
        <div class="fuzz-label">schema delta</div>
        <pre><code>old: retries is integer
new: retries is integer &lt; 5</code></pre>
        <div class="fuzz-footnote">The rule is obvious. The rollout interaction is not.</div>
      </div>

      <div class="fuzz-card fuzz-runner-card">
        <div class="fuzz-label">live search</div>
        <div class="fuzz-seed">{{ activeCase.seed }}</div>
        <pre class="fuzz-candidate"><code>{{ activeCase.candidate }}</code></pre>
        <div class="fuzz-progress" aria-hidden="true">
          <span
            v-for="n in 14"
            :key="n"
            class="fuzz-bar"
            :class="{ active: n <= (index + 1) * 3 }"
          />
        </div>
        <div class="fuzz-note">{{ activeCase.note }}</div>
      </div>

      <div class="fuzz-card fuzz-result-card">
        <div class="fuzz-label">what humans miss in review</div>
        <div class="fuzz-result-main">
          <div class="fuzz-result-line ok">writer v1 emits <code>5</code></div>
          <div class="fuzz-result-arrow">→</div>
          <div class="fuzz-result-line bad">reader v2 rejects <code>5</code></div>
        </div>
        <div class="fuzz-punchline">
          The tool does not get tired, overconfident, or distracted by how small the diff looks.
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.fuzz-demo {
  width: min(880px, 100%);
  margin: 0 auto;
}

.fuzz-topline {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  margin-bottom: 1.1rem;
}

.fuzz-status {
  padding: 0.45rem 0.8rem;
  border-radius: 999px;
  font: 700 0.84rem "IBM Plex Mono", monospace;
  text-transform: uppercase;
  letter-spacing: 0.08em;
}

.fuzz-status-searching {
  color: #7c5d10;
  background: rgba(250, 204, 21, 0.22);
}

.fuzz-status-interesting {
  color: #805ad5;
  background: rgba(196, 181, 253, 0.28);
}

.fuzz-status-breaking {
  color: #a72626;
  background: rgba(251, 113, 133, 0.22);
}

.fuzz-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 1rem;
}

.fuzz-card {
  min-height: 18.5rem;
  padding: 1rem 1.05rem;
  border-radius: 1.1rem;
  border: 1px solid var(--deck-border);
  background: rgba(255, 250, 242, 0.92);
  box-shadow: 0 14px 32px rgba(63, 40, 16, 0.08);
}

.fuzz-label {
  margin-bottom: 0.7rem;
  color: var(--deck-muted);
  font-size: 0.8rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.fuzz-card pre {
  margin: 0;
  padding: 0.95rem 1rem;
  border-radius: 0.9rem;
  background: #131820;
  color: #f8fafc;
  font-size: 0.96rem;
  line-height: 1.5;
  white-space: pre-wrap;
}

.fuzz-footnote,
.fuzz-note,
.fuzz-punchline {
  margin-top: 0.9rem;
  color: var(--deck-muted);
  line-height: 1.42;
}

.fuzz-runner-card {
  position: relative;
  overflow: hidden;
}

.fuzz-runner-card::after {
  content: "";
  position: absolute;
  inset: auto -20% -35% auto;
  width: 12rem;
  height: 12rem;
  border-radius: 50%;
  background: radial-gradient(circle, rgba(74, 222, 128, 0.18), transparent 65%);
}

.fuzz-seed {
  margin-bottom: 0.7rem;
  font: 700 1.25rem "Source Serif 4", serif;
}

.fuzz-candidate {
  min-height: 5.6rem;
}

.fuzz-progress {
  display: flex;
  gap: 0.35rem;
  margin-top: 1rem;
}

.fuzz-bar {
  width: 0.8rem;
  height: 0.7rem;
  border-radius: 999px;
  background: rgba(100, 90, 80, 0.18);
  transition: background-color 180ms ease, transform 180ms ease;
}

.fuzz-bar.active {
  background: #4ade80;
  transform: translateY(-2px);
}

.fuzz-result-main {
  margin-top: 0.5rem;
  display: grid;
  gap: 0.7rem;
}

.fuzz-result-line {
  padding: 0.9rem 1rem;
  border-radius: 0.9rem;
  font-size: 1.12rem;
  font-weight: 650;
}

.fuzz-result-line.ok {
  background: rgba(34, 197, 94, 0.14);
  border: 1px solid rgba(34, 197, 94, 0.22);
}

.fuzz-result-line.bad {
  background: rgba(251, 113, 133, 0.14);
  border: 1px solid rgba(251, 113, 133, 0.22);
}

.fuzz-result-arrow {
  text-align: center;
  color: var(--deck-accent);
  font-size: 1.8rem;
  line-height: 1;
}

@media (max-width: 900px) {
  .fuzz-topline {
    display: grid;
  }

  .fuzz-grid {
    grid-template-columns: 1fr;
  }
}
</style>
