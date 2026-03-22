<script setup lang="ts">
interface StackedBar {
  old: number;
  next: number;
}

const requestBars = Array.from({ length: 38 }, (_, index): StackedBar => {
  const rolloutDip = 19 * Math.exp(-((index - 11) ** 2) / 28);
  const trafficBump = 11 * Math.exp(-((index - 19) ** 2) / 26);
  const old = 24 - rolloutDip + 8 * Math.exp(-((index - 21) ** 2) / 40);
  const total = 24 + trafficBump + 3 * Math.exp(-((index - 8) ** 2) / 18);
  return {
    old: Math.max(3, old),
    next: Math.max(0, total - old),
  };
});

const errorBars = Array.from({ length: 38 }, (_, index): number => {
  const early = 10 * Math.exp(-((index - 7) ** 2) / 12);
  const peak = 38 * Math.exp(-((index - 19) ** 2) / 32);
  const tail = 3 * Math.exp(-((index - 29) ** 2) / 18);
  return Math.max(0.2, early + peak + tail);
});

const maxRequest = Math.max(...requestBars.map((bar) => bar.old + bar.next));
const maxError = Math.max(...errorBars);
const mixedFleet = ["old", "old", "new", "old", "new", "old"] as const;
</script>

<template>
  <div class="incident-sketch">
    <div class="incident-charts">
      <section class="sketch-panel">
        <div class="sketch-title">requests by version</div>
        <div class="sketch-plot sketch-plot-requests" aria-hidden="true">
          <div
            v-for="(bar, index) in requestBars"
            :key="`request-${index}`"
            class="sketch-bar-stack"
          >
            <div
              class="sketch-bar sketch-bar-old"
              :style="{ height: `${(bar.old / maxRequest) * 100}%` }"
            />
            <div
              class="sketch-bar sketch-bar-next"
              :style="{ height: `${(bar.next / maxRequest) * 100}%` }"
            />
          </div>
        </div>
        <div class="sketch-legend">
          <span class="legend-item legend-old">old version</span>
          <span class="legend-item legend-next">new version</span>
        </div>
      </section>

      <section class="sketch-panel">
        <div class="sketch-title">errors by version</div>
        <div class="sketch-plot sketch-plot-errors" aria-hidden="true">
          <div
            v-for="(bar, index) in errorBars"
            :key="`error-${index}`"
            class="sketch-bar sketch-bar-error"
            :style="{ height: `${(bar / maxError) * 100}%` }"
          />
        </div>
        <div class="sketch-legend">
          <span class="legend-item legend-error">parse failures</span>
        </div>
      </section>
    </div>

    <section class="sketch-panel sketch-mechanism">
      <div class="sketch-title">why this spread</div>

      <div class="pod-row" aria-label="Mixed pod versions during rollout">
        <div
          v-for="(version, index) in mixedFleet"
          :key="`pod-${index}`"
          :class="['pod-pill', version === 'new' ? 'pod-new' : 'pod-old']"
          :aria-label="`${version} pod`"
        />
        <div class="pod-note">not all pods switched at once</div>
      </div>

      <div class="cache-flow">
        <div class="flow-node flow-node-new">new pod writes<br>wrapped entry</div>
        <div class="flow-arrow">-></div>
        <div class="flow-node flow-node-cache">shared cache<br>persists across readers</div>
        <div class="flow-arrow">-></div>
        <div class="flow-node flow-node-old">old pod reads<br>parse failure</div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.incident-sketch {
  display: grid;
  gap: 0.7rem;
}

.incident-charts {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.8rem;
}

.sketch-panel {
  padding: 0.7rem 0.9rem 0.65rem;
  border: 2px solid rgba(100, 90, 80, 0.24);
  border-radius: 1rem 1.2rem 0.95rem 1.1rem;
  background: rgba(255, 251, 245, 0.9);
  box-shadow: 0 10px 24px rgba(63, 40, 16, 0.06);
  transform: rotate(-0.2deg);
}

.sketch-panel:nth-child(2) {
  transform: rotate(0.25deg);
}

.sketch-title {
  margin-bottom: 0.45rem;
  color: var(--deck-muted);
  font: 700 0.82rem "IBM Plex Mono", monospace;
  letter-spacing: 0.14em;
  text-transform: uppercase;
}

.sketch-plot {
  position: relative;
  display: flex;
  align-items: flex-end;
  gap: 0.18rem;
  height: 8.4rem;
  padding: 0.65rem 0.65rem 0.5rem;
  border: 1px dashed rgba(100, 90, 80, 0.28);
  border-radius: 0.9rem;
  background:
    repeating-linear-gradient(
      to top,
      transparent 0,
      transparent 2rem,
      rgba(100, 90, 80, 0.07) 2rem,
      rgba(100, 90, 80, 0.07) calc(2rem + 1px)
    );
}

.sketch-window {
  position: absolute;
  top: 0.6rem;
  left: 50%;
  transform: translateX(-50%) rotate(-0.8deg);
  padding: 0.2rem 0.55rem;
  border: 1px dashed rgba(173, 46, 36, 0.4);
  border-radius: 999px;
  color: var(--deck-accent);
  background: rgba(255, 247, 245, 0.92);
  font-size: 0.78rem;
  font-weight: 700;
}

.sketch-bar-stack {
  display: flex;
  flex: 1;
  flex-direction: column;
  justify-content: flex-end;
  min-width: 0;
  height: 100%;
}

.sketch-bar {
  border-radius: 2px 2px 0 0;
  min-height: 1px;
}

.sketch-bar-old {
  background: rgba(96, 165, 250, 0.86);
}

.sketch-bar-next {
  background: rgba(147, 197, 253, 0.7);
}

.sketch-bar-error {
  flex: 1;
  background: linear-gradient(to top, rgba(234, 88, 12, 0.92), rgba(251, 146, 60, 0.88));
  transform: rotate(var(--tilt, 0deg));
}

.sketch-plot-errors .sketch-bar-error:nth-child(3n) {
  --tilt: -0.4deg;
}

.sketch-plot-errors .sketch-bar-error:nth-child(4n) {
  --tilt: 0.35deg;
}

.sketch-legend {
  display: flex;
  gap: 0.9rem;
  margin-top: 0.45rem;
  color: var(--deck-muted);
  font-size: 0.86rem;
}

.legend-item::before {
  content: "";
  display: inline-block;
  width: 0.8rem;
  height: 0.8rem;
  margin-right: 0.4rem;
  border-radius: 0.2rem;
  vertical-align: -0.12rem;
}

.legend-old::before {
  background: rgba(96, 165, 250, 0.86);
}

.legend-next::before {
  background: rgba(147, 197, 253, 0.7);
}

.legend-error::before {
  background: rgba(234, 88, 12, 0.92);
}

.sketch-mechanism {
  transform: rotate(-0.1deg);
}

.pod-row {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 0.45rem;
}

.pod-pill {
  padding: 0.25rem 0.6rem;
  border-radius: 999px;
  font: 700 0.9rem "IBM Plex Mono", monospace;
}

.pod-old {
  border: 1px solid rgba(100, 116, 139, 0.3);
  background: rgba(241, 245, 249, 0.95);
  color: #475569;
}

.pod-new {
  border: 1px solid rgba(37, 99, 235, 0.28);
  background: rgba(219, 234, 254, 0.95);
  color: #1d4ed8;
}

.pod-note {
  margin-left: 0.6rem;
  color: var(--deck-muted);
  font-size: 0.98rem;
  font-style: italic;
}

.cache-flow {
  display: grid;
  grid-template-columns: 1fr auto 1.3fr auto 1fr;
  align-items: center;
  gap: 0.6rem;
  margin-top: 0.6rem;
}

.flow-node {
  padding: 0.55rem 0.7rem;
  border-radius: 0.9rem;
  text-align: center;
  font-size: 0.95rem;
  line-height: 1.25;
}

.flow-node-new {
  border: 1px solid rgba(37, 99, 235, 0.25);
  background: rgba(219, 234, 254, 0.72);
}

.flow-node-cache {
  border: 1px solid rgba(187, 139, 46, 0.25);
  background: rgba(254, 243, 199, 0.78);
}

.flow-node-old {
  border: 1px solid rgba(173, 46, 36, 0.24);
  background: rgba(254, 226, 226, 0.68);
}

.flow-arrow {
  color: var(--deck-muted);
  font: 700 1.2rem "IBM Plex Mono", monospace;
}

@media (max-width: 900px) {
  .incident-charts,
  .cache-flow {
    grid-template-columns: 1fr;
  }

  .flow-arrow {
    text-align: center;
  }
}
</style>
