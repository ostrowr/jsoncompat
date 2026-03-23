<script setup lang="ts">
interface StackedBar {
  old: number;
  next: number;
}

const requestBars = Array.from({ length: 28 }, (_, index): StackedBar => {
  const t = index / 27;
  const total = 28 + 0.8 * Math.sin(t * Math.PI);
  const oldShare = 1 - 0.5 * t ** 1.05;
  return {
    old: total * oldShare,
    next: total * (1 - oldShare),
  };
});

const errorBars = Array.from({ length: 28 }, (_, index): number => {
  const t = index / 27;
  return 1.5 + 34 * t ** 1.65;
});

const maxRequest = Math.max(...requestBars.map((bar) => bar.old + bar.next));
const maxError = Math.max(...errorBars);
</script>

<template>
  <div class="audience-rollout-question">
    <div class="question-copy">
      <h1>Errors are climbing during a deploy.</h1>
      <p class="deck-lead deck-muted">What do you do?</p>
    </div>

    <div class="question-charts">
      <section class="question-panel">
        <div class="question-title">requests by version</div>
        <div class="question-plot" aria-hidden="true">
          <div
            v-for="(bar, index) in requestBars"
            :key="`request-${index}`"
            class="question-bar-stack"
          >
            <div
              class="question-bar question-bar-old"
              :style="{ height: `${(bar.old / maxRequest) * 100}%` }"
            />
            <div
              class="question-bar question-bar-next"
              :style="{ height: `${(bar.next / maxRequest) * 100}%` }"
            />
          </div>
        </div>
      </section>

      <section class="question-panel">
        <div class="question-title">errors</div>
        <div class="question-plot" aria-hidden="true">
          <div
            v-for="(bar, index) in errorBars"
            :key="`error-${index}`"
            class="question-bar question-bar-error"
            :style="{ height: `${(bar / maxError) * 100}%` }"
          />
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped>
.audience-rollout-question {
  display: grid;
  align-content: start;
  gap: 0.8rem;
  min-height: 0;
}

.question-copy {
  max-width: 45rem;
}

.question-copy h1 {
  margin-bottom: 0.35rem;
  font-size: 2.95rem;
  line-height: 1.02;
}

.question-copy .deck-lead {
  margin: 0;
}

.question-charts {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.9rem;
}

.question-panel {
  padding: 0.8rem 1rem 0.8rem;
  border: 1px solid rgba(148, 163, 184, 0.24);
  border-radius: 0.35rem;
  background: rgba(6, 13, 22, 0.82);
}

.question-title {
  margin-bottom: 0.5rem;
  color: #bcc7d4;
  font: 700 0.82rem "IBM Plex Mono", monospace;
  letter-spacing: 0.14em;
  text-transform: uppercase;
}

.question-plot {
  display: flex;
  align-items: flex-end;
  gap: 0.22rem;
  height: 10.4rem;
  padding: 0.65rem 0.65rem 0.45rem;
  border: 1px solid rgba(148, 163, 184, 0.18);
  background:
    repeating-linear-gradient(
      to top,
      transparent 0,
      transparent 2.8rem,
      rgba(148, 163, 184, 0.12) 2.8rem,
      rgba(148, 163, 184, 0.12) calc(2.8rem + 1px)
    );
}

.question-bar-stack {
  display: flex;
  flex: 1;
  flex-direction: column;
  justify-content: flex-end;
  min-width: 0;
  height: 100%;
}

.question-bar {
  flex: 0 0 auto;
  min-height: 1px;
  border-radius: 2px 2px 0 0;
}

.question-bar-old {
  background: linear-gradient(to top, rgba(37, 99, 235, 0.92), rgba(96, 165, 250, 0.9));
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.18);
}

.question-bar-next {
  background: rgba(147, 197, 253, 0.72);
  border-top: 1px solid rgba(241, 245, 249, 0.22);
}

.question-bar-error {
  flex: 1;
  background: linear-gradient(to top, rgba(255, 93, 61, 0.95), rgba(255, 138, 114, 0.92));
}

@media (max-width: 900px) {
  .question-charts {
    grid-template-columns: 1fr;
  }

  .question-plot {
    height: 8rem;
  }
}
</style>
