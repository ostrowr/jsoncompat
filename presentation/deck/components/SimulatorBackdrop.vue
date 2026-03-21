<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, shallowRef } from "vue";
import {
  startInteractiveApp,
  type InteractiveAppHandle,
} from "@interactive/app";

const props = withDefaults(defineProps<{
  startStateId?: string;
  emitRatePerSec?: number;
}>(), {
  startStateId: "s7",
  emitRatePerSec: 0.34,
});

const mountEl = ref<HTMLElement | null>(null);
const handle = shallowRef<InteractiveAppHandle | null>(null);

onMounted(async () => {
  if (mountEl.value === null) {
    return;
  }

  handle.value = await startInteractiveApp(mountEl.value, {
    initialStateId: props.startStateId,
    startPaused: false,
    enableKeyboard: false,
    initialEmitRatePerSec: props.emitRatePerSec,
    resizeTarget: window,
    chrome: {
      showEmissionControl: false,
      showPauseButton: false,
      showStateChip: false,
    },
    scene: {
      backgroundAlpha: 0,
      panelAlpha: 0.08,
      wireAlpha: 0.72,
      packetAlpha: 0.96,
      effectsAlpha: 0.62,
    },
  });
});

onBeforeUnmount(() => {
  handle.value?.dispose();
});
</script>

<template>
  <div class="deck-backdrop-layer" aria-hidden="true">
    <div class="deck-backdrop-frame">
      <div ref="mountEl" class="deck-backdrop-sim" />
    </div>
    <div class="deck-backdrop-fade" />
  </div>
</template>

<style scoped>
.deck-backdrop-layer {
  position: fixed;
  inset: 0;
  overflow: hidden;
  pointer-events: none;
  z-index: 0;
}

.deck-backdrop-frame {
  position: absolute;
  inset: 0;
  overflow: hidden;
}

.deck-backdrop-sim {
  position: absolute;
  left: 50%;
  top: 50%;
  width: 100vw;
  height: 100vh;
  opacity: 0.94;
  filter: saturate(1.1) contrast(1.03) brightness(0.94) blur(0.35px);
  transform: translate(-50%, -50%) scale(1.24);
  transform-origin: center;
}

.deck-backdrop-sim :deep(canvas) {
  width: 100% !important;
  height: 100% !important;
}

.deck-backdrop-fade {
  position: absolute;
  inset: 0;
  background:
    radial-gradient(circle at 50% 48%, rgba(245, 239, 226, 0.04), rgba(245, 239, 226, 0.18) 64%),
    linear-gradient(180deg, rgba(245, 239, 226, 0.12) 0%, rgba(245, 239, 226, 0.03) 26%, rgba(245, 239, 226, 0.18) 100%);
}
</style>
