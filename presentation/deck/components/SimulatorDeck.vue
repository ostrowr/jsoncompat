<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, shallowRef } from "vue";
import {
  startInteractiveApp,
  type InteractiveAppHandle,
} from "@interactive/app";

type DeckMode = "steady" | "transition" | "paused";

const props = withDefaults(defineProps<{
  title: string;
  caption: string;
  takeaway?: string;
  mode?: DeckMode;
  startStateId?: string;
  sequence?: string[];
  stepDelayMs?: number;
  autoplay?: boolean;
  pauseAtEnd?: boolean;
  emitRatePerSec?: number;
  height?: string;
  showFooter?: boolean;
  showControls?: boolean;
  showTag?: boolean;
  fullscreen?: boolean;
  layoutScale?: number;
  bare?: boolean;
  showStateChip?: boolean;
  packetSpeedPxPerSec?: number;
  initialPacketCount?: number;
  initialPacketSpacingPx?: number;
  minimumPacketGapPx?: number;
}>(), {
  takeaway: "",
  mode: "steady",
  startStateId: "s1",
  sequence: () => [],
  stepDelayMs: 1800,
  autoplay: false,
  pauseAtEnd: false,
  emitRatePerSec: 0.62,
  height: "560px",
  showFooter: true,
  showControls: true,
  showTag: true,
  fullscreen: false,
  layoutScale: 1,
  bare: false,
  showStateChip: true,
  packetSpeedPxPerSec: undefined,
  initialPacketCount: undefined,
  initialPacketSpacingPx: undefined,
  minimumPacketGapPx: undefined,
});

const modeLabel = computed(() => {
  if (props.mode === "paused") {
    return "Freeze-frame analysis";
  }
  if (props.mode === "transition") {
    return "Scripted demo beat";
  }
  return "Live wire state";
});

const mountEl = ref<HTMLElement | null>(null);
const handle = shallowRef<InteractiveAppHandle | null>(null);
const currentStateId = ref(props.startStateId);
const paused = ref(props.mode === "paused");
const running = ref(false);
const sequenceIndex = ref(0);
const timeouts: number[] = [];
let resizeObserver: ResizeObserver | null = null;
let mountAttemptRafId: number | null = null;
let keydownHandler: ((event: KeyboardEvent) => void) | null = null;

const clearTimers = (): void => {
  for (const timerId of timeouts.splice(0)) {
    window.clearTimeout(timerId);
  }
  running.value = false;
};

const syncState = (): void => {
  currentStateId.value = handle.value?.stateId() ?? props.startStateId;
};

const resetDemo = (): void => {
  clearTimers();
  handle.value?.reset();
  syncState();
  sequenceIndex.value = 0;
  const shouldPause = props.mode === "paused";
  handle.value?.setPaused(shouldPause);
  paused.value = shouldPause;
};

const togglePause = (): void => {
  if (handle.value === null) {
    return;
  }
  const next = !paused.value;
  handle.value.setPaused(next);
  paused.value = next;
};

const runBeat = (): void => {
  if (handle.value === null) {
    return;
  }
  resetDemo();
  handle.value.setPaused(false);
  paused.value = false;
  if (props.sequence.length === 0) {
    return;
  }
  running.value = true;
  props.sequence.forEach((stateId, index) => {
    const timerId = window.setTimeout(() => {
      handle.value?.transitionTo(stateId);
      syncState();
      if (index === props.sequence.length - 1) {
        running.value = false;
        if (props.pauseAtEnd) {
          handle.value?.setPaused(true);
          paused.value = true;
        }
      }
    }, props.stepDelayMs * (index + 1));
    timeouts.push(timerId);
  });
};

const advanceSequence = (): void => {
  if (handle.value === null || props.mode !== "transition") {
    return;
  }
  if (sequenceIndex.value >= props.sequence.length) {
    return;
  }
  handle.value.setPaused(false);
  paused.value = false;
  handle.value.transitionTo(props.sequence[sequenceIndex.value] ?? props.startStateId);
  syncState();
  sequenceIndex.value += 1;
  if (sequenceIndex.value >= props.sequence.length && props.pauseAtEnd) {
    handle.value.setPaused(true);
    paused.value = true;
  }
};

const startApp = async (): Promise<void> => {
  if (mountEl.value === null || handle.value !== null) {
    return;
  }

  handle.value = await startInteractiveApp(mountEl.value, {
    initialStateId: props.startStateId,
    startPaused: props.mode === "paused",
    enableKeyboard: false,
    initialEmitRatePerSec: props.emitRatePerSec,
    packetSpeedPxPerSec: props.packetSpeedPxPerSec,
    initialPacketCount: props.initialPacketCount,
    initialPacketSpacingPx: props.initialPacketSpacingPx,
    minimumPacketGapPx: props.minimumPacketGapPx,
    resizeTarget: mountEl.value,
    chrome: {
      showEmissionControl: false,
      showPauseButton: false,
      showStateChip: props.showStateChip,
    },
    scene: {
      layoutScale: props.layoutScale,
    },
  });
  syncState();
  paused.value = props.mode === "paused";

  if (props.mode === "transition" && props.autoplay) {
    runBeat();
  }
};

const cancelMountAttempt = (): void => {
  if (mountAttemptRafId !== null) {
    window.cancelAnimationFrame(mountAttemptRafId);
    mountAttemptRafId = null;
  }
};

const ensureMountedWhenVisible = async (): Promise<void> => {
  cancelMountAttempt();
  await nextTick();
  const attempt = async (): Promise<void> => {
    const element = mountEl.value;
    if (element === null) {
      return;
    }
    if (element.clientWidth > 0 && element.clientHeight > 0) {
      if (handle.value === null) {
        await startApp();
      } else {
        handle.value.refreshLayout();
      }
      return;
    }
    mountAttemptRafId = window.requestAnimationFrame(() => {
      void attempt();
    });
  };
  await attempt();
};

onMounted(async () => {
  if (mountEl.value === null) {
    return;
  }

  resizeObserver = new ResizeObserver(() => {
    if (mountEl.value === null) {
      return;
    }
    if (mountEl.value.clientWidth <= 0 || mountEl.value.clientHeight <= 0) {
      return;
    }
    if (handle.value === null) {
      void ensureMountedWhenVisible();
      return;
    }
    handle.value.refreshLayout();
  });
  resizeObserver.observe(mountEl.value);
  keydownHandler = (event: KeyboardEvent) => {
    if (mountEl.value === null || mountEl.value.clientWidth <= 0 || mountEl.value.clientHeight <= 0) {
      return;
    }
    const activeElement = document.activeElement;
    if (
      activeElement instanceof HTMLInputElement ||
      activeElement instanceof HTMLTextAreaElement ||
      activeElement instanceof HTMLSelectElement ||
      activeElement?.getAttribute("contenteditable") === "true"
    ) {
      return;
    }

    if (event.key === "s" || event.key === "S") {
      if (props.mode !== "transition") {
        return;
      }
      event.preventDefault();
      advanceSequence();
      return;
    }

    if (event.key === "r" || event.key === "R") {
      event.preventDefault();
      resetDemo();
      return;
    }

    if (event.key === "p" || event.key === "P") {
      event.preventDefault();
      togglePause();
    }
  };
  window.addEventListener("keydown", keydownHandler);
  await ensureMountedWhenVisible();
});

onBeforeUnmount(() => {
  clearTimers();
  cancelMountAttempt();
  resizeObserver?.disconnect();
  if (keydownHandler !== null) {
    window.removeEventListener("keydown", keydownHandler);
  }
  handle.value?.dispose();
});
</script>

<template>
  <div
    class="sim-shell"
    :class="{
      'deck-panel': !fullscreen,
      'sim-shell-fullscreen': fullscreen,
      'sim-shell-bare': bare,
    }"
    :style="{ '--sim-height': height }"
    :data-demo-title="title"
  >
    <div v-if="!bare" class="sim-copy">
      <div v-if="showTag" class="sim-tag">{{ modeLabel }}</div>
      <h3>{{ title }}</h3>
      <p>{{ caption }}</p>
      <p v-if="takeaway" class="sim-takeaway">{{ takeaway }}</p>
    </div>

    <div ref="mountEl" class="sim-stage" />

    <div v-if="bare && mode === 'transition'" class="sim-bare-hint">
      <span><kbd>s</kbd> next state</span>
      <span><kbd>r</kbd> reset</span>
      <span><kbd>p</kbd> pause</span>
    </div>

    <div v-if="showFooter && !bare" class="sim-footer">
      <div class="sim-state">
        <span class="sim-pill">state {{ currentStateId }}</span>
        <span class="sim-pill">{{ paused ? "paused" : "flowing" }}</span>
      </div>

      <div v-if="showControls" class="sim-controls">
        <button
          v-if="mode === 'transition'"
          data-testid="run-beat"
          type="button"
          class="sim-button sim-button-primary"
          @click="runBeat"
        >
          {{ running ? "Running beat…" : "Run beat" }}
        </button>
        <button data-testid="reset-demo" type="button" class="sim-button" @click="resetDemo">
          Reset
        </button>
        <button data-testid="previous-state" type="button" class="sim-button" @click="handle?.previousState(); syncState()">
          Back
        </button>
        <button data-testid="next-state" type="button" class="sim-button" @click="handle?.nextState(); syncState()">
          Next
        </button>
        <button data-testid="toggle-pause" type="button" class="sim-button" @click="togglePause">
          {{ paused ? "Resume" : "Pause" }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.sim-shell {
  display: grid;
  gap: 1rem;
  padding: 1rem;
}

.sim-shell-bare {
  display: block;
  width: 100%;
  position: relative;
  padding: 0;
  background: transparent;
  border: 0;
  box-shadow: none;
  backdrop-filter: none;
}

.sim-shell-fullscreen {
  width: 100%;
  max-width: none;
  min-height: auto;
  padding: 0.95rem 0.95rem 1rem;
  border-radius: 1.35rem;
  border: 1px solid rgba(41, 27, 12, 0.12);
  background: rgba(255, 249, 240, 0.76);
  box-shadow: 0 18px 54px rgba(63, 40, 16, 0.12);
  backdrop-filter: blur(10px);
  margin: 0 auto;
}

.sim-copy {
  display: grid;
  gap: 0.45rem;
}

.sim-copy h3 {
  margin: 0;
  font-size: 1.4rem;
}

.sim-copy p {
  margin: 0;
  color: #584d43;
}

.sim-tag {
  font-size: 0.76rem;
  font-weight: 700;
  letter-spacing: 0.09em;
  text-transform: uppercase;
  color: #ad2e24;
}

.sim-takeaway {
  padding: 0.65rem 0.8rem;
  border-radius: 0.85rem;
  background: rgba(173, 46, 36, 0.08);
  border: 1px solid rgba(173, 46, 36, 0.14);
  color: #74231d;
  font-weight: 600;
}

.sim-stage {
  min-height: var(--sim-height);
  height: var(--sim-height);
  border-radius: 1rem;
  overflow: hidden;
  border: 1px solid rgba(20, 29, 44, 0.15);
  box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.04);
}

.sim-shell-bare .sim-stage {
  min-height: min(var(--sim-height), 540px);
  height: min(var(--sim-height), 540px);
  width: 100%;
  border: 0;
  border-radius: 0;
  box-shadow: none;
}

.sim-bare-hint {
  position: absolute;
  right: 1rem;
  bottom: 1rem;
  display: flex;
  gap: 0.7rem;
  padding: 0.45rem 0.7rem;
  border-radius: 999px;
  background: rgba(7, 13, 23, 0.7);
  color: rgba(232, 237, 247, 0.9);
  font-family: "IBM Plex Mono", monospace;
  font-size: 0.74rem;
  letter-spacing: 0.01em;
}

.sim-bare-hint kbd {
  display: inline-block;
  min-width: 1.4rem;
  margin-right: 0.35rem;
  padding: 0.08rem 0.3rem;
  border-radius: 0.35rem;
  background: rgba(255, 255, 255, 0.12);
  font-family: inherit;
  text-align: center;
}

.sim-shell-fullscreen .sim-stage {
  min-height: min(60vh, 620px);
  height: min(60vh, 620px);
}

.sim-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 1rem;
  flex-wrap: wrap;
}

.sim-shell-fullscreen .sim-copy {
  gap: 0.28rem;
}

.sim-shell-fullscreen .sim-copy h3 {
  font-size: 1.7rem;
}

.sim-shell-fullscreen .sim-copy p {
  font-size: 1rem;
}

.sim-shell-fullscreen .sim-takeaway {
  padding: 0.5rem 0.7rem;
}

.sim-state,
.sim-controls {
  display: flex;
  gap: 0.6rem;
  flex-wrap: wrap;
}

.sim-pill {
  padding: 0.4rem 0.72rem;
  border-radius: 999px;
  background: rgba(255, 251, 246, 0.85);
  border: 1px solid rgba(71, 51, 32, 0.12);
  font-size: 0.84rem;
  color: #5a5148;
}

.sim-button {
  appearance: none;
  border: 1px solid rgba(55, 42, 27, 0.14);
  border-radius: 999px;
  background: rgba(255, 252, 249, 0.94);
  color: #1d1d1f;
  padding: 0.5rem 0.9rem;
  font-size: 0.9rem;
  font-weight: 600;
  cursor: pointer;
  transition: transform 120ms ease, background 120ms ease;
}

.sim-button:hover {
  transform: translateY(-1px);
  background: rgba(255, 255, 255, 1);
}

.sim-button-primary {
  background: #ad2e24;
  color: #fff8f0;
  border-color: transparent;
}
</style>
