<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";

const props = withDefaults(defineProps<{
  flashTone?: "success" | "failure";
  redReceiveEvery?: number;
  titleLayout?: boolean;
  hiddenNodeIds?: readonly string[];
}>(), {
  flashTone: "success",
  redReceiveEvery: 0,
  titleLayout: false,
  hiddenNodeIds: () => [],
});

interface NetworkNode {
  id: string;
  label: string;
  x: number;
  y: number;
  role: "core" | "service" | "edge";
}

interface PacketRoute {
  id: string;
  from: string;
  to: string;
  offsetSec: number;
  travelSec: number;
  idleSec: number;
  size: number;
  color: string;
}

interface RenderPacket {
  id: string;
  x: number;
  y: number;
  size: number;
  color: string;
  visible: boolean;
}

interface TrailPoint {
  id: string;
  x: number;
  y: number;
  color: string;
  ttlSec: number;
}

interface RenderTrailPoint {
  id: string;
  x: number;
  y: number;
  color: string;
  opacity: number;
  size: number;
}

type FlashTone = "success" | "failure";

interface NodeFlash {
  ttlSec: number;
  tone: FlashTone;
}

const TRAIL_TTL_SEC = 0.42;
const TRAIL_SPACING_PCT = 1.35;

const defaultNodes = [
  { id: "edge-usw", label: "edge", x: 10, y: 22, role: "edge" },
  { id: "edge-use", label: "edge", x: 87, y: 18, role: "edge" },
  { id: "api", label: "api", x: 24, y: 48, role: "service" },
  { id: "queue", label: "queue", x: 40, y: 24, role: "service" },
  { id: "router", label: "router", x: 48, y: 50, role: "core" },
  { id: "worker", label: "worker", x: 67, y: 28, role: "service" },
  { id: "cache", label: "cache", x: 18, y: 76, role: "service" },
  { id: "db", label: "db", x: 84, y: 57, role: "service" },
  { id: "analytics", label: "analytics", x: 47, y: 85, role: "service" },
  { id: "search", label: "search", x: 76, y: 88, role: "service" },
] as const satisfies readonly NetworkNode[];

const titleNodes = [
  { id: "edge-usw", label: "edge", x: 10, y: 22, role: "edge" },
  { id: "edge-use", label: "edge", x: 87, y: 18, role: "edge" },
  { id: "api", label: "api", x: 24, y: 48, role: "service" },
  { id: "queue", label: "queue", x: 40, y: 24, role: "service" },
  { id: "router", label: "router", x: 48, y: 50, role: "core" },
  { id: "worker", label: "worker", x: 67, y: 28, role: "service" },
  { id: "cache", label: "cache", x: 18, y: 76, role: "service" },
  { id: "db", label: "db", x: 84, y: 57, role: "service" },
  { id: "analytics", label: "analytics", x: 47, y: 85, role: "service" },
  { id: "search", label: "search", x: 76, y: 88, role: "service" },
] as const satisfies readonly NetworkNode[];

const nodes = computed<readonly NetworkNode[]>(() => props.titleLayout ? titleNodes : defaultNodes);
const nodeById = computed(() => new Map(nodes.value.map((node) => [node.id, node] as const)));
const visibleNodes = computed<readonly NetworkNode[]>(() => {
  const hidden = new Set(props.hiddenNodeIds);
  return nodes.value.filter((node) => !hidden.has(node.id));
});

const routes = [
  { id: "p1", from: "edge-usw", to: "api", offsetSec: 0.0, travelSec: 1.4, idleSec: 2.3, size: 9, color: "#8dd3ff" },
  { id: "p2", from: "api", to: "router", offsetSec: 0.6, travelSec: 1.1, idleSec: 2.6, size: 8, color: "#8dd3ff" },
  { id: "p3", from: "router", to: "worker", offsetSec: 1.5, travelSec: 1.5, idleSec: 2.2, size: 9, color: "#c9a7ff" },
  { id: "p4", from: "worker", to: "db", offsetSec: 2.5, travelSec: 1.3, idleSec: 2.4, size: 8, color: "#8dd3ff" },
  { id: "p5", from: "edge-use", to: "worker", offsetSec: 0.8, travelSec: 1.8, idleSec: 2.0, size: 7, color: "#ffd166" },
  { id: "p6", from: "router", to: "cache", offsetSec: 0.3, travelSec: 1.2, idleSec: 2.7, size: 8, color: "#8dd3ff" },
  { id: "p8", from: "api", to: "queue", offsetSec: 2.1, travelSec: 1.0, idleSec: 2.8, size: 7, color: "#c9a7ff" },
  { id: "p9", from: "queue", to: "router", offsetSec: 3.0, travelSec: 1.2, idleSec: 2.3, size: 7, color: "#ffd166" },
  { id: "p10", from: "router", to: "analytics", offsetSec: 3.6, travelSec: 1.7, idleSec: 2.4, size: 8, color: "#c9a7ff" },
  { id: "p11", from: "edge-usw", to: "queue", offsetSec: 4.3, travelSec: 1.7, idleSec: 2.2, size: 7, color: "#8dd3ff" },
  { id: "p12", from: "edge-use", to: "db", offsetSec: 5.0, travelSec: 2.2, idleSec: 2.8, size: 9, color: "#ffd166" },
  { id: "p14", from: "router", to: "search", offsetSec: 2.8, travelSec: 1.8, idleSec: 2.5, size: 8, color: "#ffd166" },
] as const satisfies readonly PacketRoute[];

const activeRoutes = computed<readonly PacketRoute[]>(() => {
  const hidden = new Set(props.hiddenNodeIds);
  return routes.filter((route) => !hidden.has(route.from) && !hidden.has(route.to));
});

const packets = ref<RenderPacket[]>(
  activeRoutes.value.map((route) => ({
    id: route.id,
    x: nodeById.value.get(route.from)?.x ?? 0,
    y: nodeById.value.get(route.from)?.y ?? 0,
    size: route.size,
    color: route.color,
    visible: true,
  })),
);
const flashByNodeId = ref<Record<string, NodeFlash>>({});
const trailPoints = ref<RenderTrailPoint[]>([]);
let receiveCount = 0;

const pointFor = (id: string): NetworkNode => {
  const node = nodeById.value.get(id);
  if (node === undefined) {
    throw new Error(`unknown network node '${id}'`);
  }
  return node;
};

const lerp = (from: number, to: number, t: number): number => from + (to - from) * t;
const mod = (value: number, divisor: number): number => ((value % divisor) + divisor) % divisor;
const distancePct = (a: { x: number; y: number }, b: { x: number; y: number }): number => {
  return Math.hypot(a.x - b.x, a.y - b.y);
};

const nodeClass = (node: NetworkNode): string => {
  const flash = flashByNodeId.value[node.id];
  const flashClass = flash?.tone === "failure" ? "network-node-failed" : "network-node-arrived";
  return `network-node network-node-${node.role}${flash !== undefined ? ` ${flashClass}` : ""}`;
};

const packetStyles = computed(() => {
  return packets.value.map((packet) => ({
    left: `${packet.x}%`,
    top: `${packet.y}%`,
    width: `${packet.size}px`,
    height: `${packet.size}px`,
    background: packet.color,
    opacity: packet.visible ? "1" : "0",
    boxShadow: `0 0 18px ${packet.color}`,
  }));
});

let animationFrameId: number | null = null;
let startTimeSec: number | null = null;
let lastFrameSec: number | null = null;
const previousPhaseByRouteId = new Map<string, number>();
const trailByRouteId = new Map<string, TrailPoint[]>();

const renderAt = (nowSec: number): void => {
  const elapsedSec = startTimeSec === null ? 0 : nowSec - startTimeSec;
  const deltaSec = lastFrameSec === null ? 0 : Math.min(0.1, nowSec - lastFrameSec);
  lastFrameSec = nowSec;

  const nextPackets: RenderPacket[] = [];
  const nextTrailPoints: RenderTrailPoint[] = [];
  const nextFlash: Record<string, NodeFlash> = {};
  for (const [nodeId, flash] of Object.entries(flashByNodeId.value)) {
    const remaining = flash.ttlSec - deltaSec;
    if (remaining > 0) {
      nextFlash[nodeId] = { ttlSec: remaining, tone: flash.tone };
    }
  }

  for (const route of activeRoutes.value) {
    const from = pointFor(route.from);
    const to = pointFor(route.to);
    const cycleSec = route.travelSec + route.idleSec;
    const phaseSec = mod(elapsedSec + route.offsetSec, cycleSec);
    const previousPhaseSec = previousPhaseByRouteId.get(route.id);
    if (previousPhaseSec !== undefined && previousPhaseSec < route.travelSec && phaseSec >= route.travelSec) {
      receiveCount += 1;
      const shouldFail =
        props.redReceiveEvery > 0
          ? receiveCount % props.redReceiveEvery === 0
          : props.flashTone === "failure";
      const existing = nextFlash[route.to];
      if (existing === undefined || existing.ttlSec < 0.45 || shouldFail) {
        nextFlash[route.to] = {
          ttlSec: Math.max(existing?.ttlSec ?? 0, 0.45),
          tone: shouldFail ? "failure" : "success",
        };
      }
    }
    previousPhaseByRouteId.set(route.id, phaseSec);

    const visible = phaseSec < route.travelSec;
    const progress = visible ? phaseSec / route.travelSec : 1;
    nextPackets.push({
      id: route.id,
      x: lerp(from.x, to.x, progress),
      y: lerp(from.y, to.y, progress),
      size: route.size,
      color: route.color,
      visible,
    });

    const existingTrail = trailByRouteId.get(route.id) ?? [];
    const liveTrail = existingTrail
      .map((point) => ({ ...point, ttlSec: point.ttlSec - deltaSec }))
      .filter((point) => point.ttlSec > 0);
    const currentPosition = {
      x: lerp(from.x, to.x, progress),
      y: lerp(from.y, to.y, progress),
    };
    const lastTrailPoint = liveTrail[liveTrail.length - 1];
    if (
      visible
      && (lastTrailPoint === undefined || distancePct(lastTrailPoint, currentPosition) >= TRAIL_SPACING_PCT)
    ) {
      liveTrail.push({
        id: `${route.id}-${nowSec.toFixed(3)}`,
        x: currentPosition.x,
        y: currentPosition.y,
        color: route.color,
        ttlSec: TRAIL_TTL_SEC,
      });
    }
    trailByRouteId.set(route.id, liveTrail);
    for (const point of liveTrail) {
      nextTrailPoints.push({
        id: point.id,
        x: point.x,
        y: point.y,
        color: point.color,
        opacity: Math.max(0, point.ttlSec / TRAIL_TTL_SEC) * 0.18,
        size: 2,
      });
    }
  }

  packets.value = nextPackets;
  trailPoints.value = nextTrailPoints;
  flashByNodeId.value = nextFlash;
};

const tick = (nowMs: number): void => {
  const nowSec = nowMs / 1000;
  if (startTimeSec === null) {
    startTimeSec = nowSec;
  }
  renderAt(nowSec);
  animationFrameId = window.requestAnimationFrame(tick);
};

onMounted(() => {
  const initialNowSec = performance.now() / 1000;
  startTimeSec = initialNowSec - 1.4;
  lastFrameSec = initialNowSec;
  renderAt(initialNowSec);
  animationFrameId = window.requestAnimationFrame(tick);
});

onBeforeUnmount(() => {
  if (animationFrameId !== null) {
    window.cancelAnimationFrame(animationFrameId);
  }
});
</script>

<template>
  <section class="network-hero" aria-label="Animated distributed system with packets moving between services">
    <div class="network-glow network-glow-left" />
    <div class="network-glow network-glow-right" />

    <div
      v-for="node in visibleNodes"
      :key="node.id"
      :class="nodeClass(node)"
      :style="{ left: `${node.x}%`, top: `${node.y}%` }"
    >
      <span class="network-node-dot" />
      <span class="network-node-label">{{ node.label }}</span>
    </div>

    <div
      v-for="trail in trailPoints"
      :key="trail.id"
      class="network-trail-point"
      :style="{
        left: `${trail.x}%`,
        top: `${trail.y}%`,
        width: `${trail.size}px`,
        height: `${trail.size}px`,
        background: trail.color,
        opacity: String(trail.opacity),
        boxShadow: `0 0 10px ${trail.color}`,
      }"
    />

    <div
      v-for="(packet, index) in packets"
      :key="packet.id"
      class="network-packet"
      :style="packetStyles[index]"
    />

    <div class="network-hero-copy">
      <slot />
    </div>

  </section>
</template>

<style scoped>
.network-hero {
  position: relative;
  width: 100%;
  height: 100%;
  min-height: 100%;
  overflow: hidden;
  background:
    radial-gradient(circle at 20% 20%, rgba(92, 225, 230, 0.18), transparent 28%),
    radial-gradient(circle at 78% 30%, rgba(134, 239, 172, 0.16), transparent 24%),
    linear-gradient(145deg, #06111f 0%, #0b1728 48%, #111f2f 100%);
  color: #f7fbff;
}

.network-glow {
  position: absolute;
  width: 34rem;
  height: 34rem;
  border-radius: 50%;
  filter: blur(2px);
  opacity: 0.5;
  pointer-events: none;
}

.network-glow-left {
  left: -8rem;
  top: -5rem;
  background: radial-gradient(circle, rgba(56, 189, 248, 0.25), transparent 68%);
}

.network-glow-right {
  right: -6rem;
  bottom: -8rem;
  background: radial-gradient(circle, rgba(74, 222, 128, 0.22), transparent 68%);
}

.network-node {
  position: absolute;
  width: 0;
  height: 0;
}

.network-node-dot {
  position: absolute;
  left: 50%;
  top: 50%;
  width: 18px;
  height: 18px;
  border-radius: 999px;
  border: 2px solid rgba(226, 232, 240, 0.92);
  background: rgba(15, 23, 42, 0.95);
  box-shadow: 0 0 0 8px rgba(148, 163, 184, 0.08);
  transform: translate(-50%, -50%);
  transition:
    background-color 120ms ease,
    border-color 120ms ease,
    box-shadow 120ms ease;
}

.network-node-core .network-node-dot {
  width: 24px;
  height: 24px;
  border-color: rgba(251, 191, 36, 0.95);
  box-shadow: 0 0 0 10px rgba(251, 191, 36, 0.1);
}

.network-node-edge .network-node-dot {
  border-color: rgba(147, 197, 253, 0.95);
}

.network-node-arrived .network-node-dot {
  background: rgba(74, 222, 128, 0.98);
  border-color: rgba(187, 247, 208, 1);
  box-shadow:
    0 0 0 12px rgba(74, 222, 128, 0.18),
    0 0 28px rgba(74, 222, 128, 0.95);
}

.network-node-failed .network-node-dot {
  background: rgba(251, 113, 133, 0.98);
  border-color: rgba(254, 202, 202, 1);
  box-shadow:
    0 0 0 12px rgba(251, 113, 133, 0.16),
    0 0 28px rgba(251, 113, 133, 0.92);
}

.network-node-label {
  position: absolute;
  left: 50%;
  top: 18px;
  transform: translateX(-50%);
  font: 600 0.82rem "IBM Plex Mono", monospace;
  color: rgba(241, 245, 249, 0.86);
  padding: 0.18rem 0.5rem;
  border-radius: 999px;
  background: rgba(15, 23, 42, 0.55);
  border: 1px solid rgba(148, 163, 184, 0.2);
  white-space: nowrap;
}

.network-packet {
  position: absolute;
  border-radius: 999px;
  transform: translate(-50%, -50%);
  transition: opacity 80ms linear;
}

.network-trail-point {
  position: absolute;
  border-radius: 999px;
  transform: translate(-50%, -50%);
  pointer-events: none;
  box-shadow: none !important;
}

.network-hero-copy {
  position: absolute;
  right: 4rem;
  top: 58%;
  max-width: 29rem;
  transform: translateY(-50%);
  color: rgba(226, 232, 240, 0.92);
  text-align: left;
}

.network-hero-copy :deep(h1) {
  margin: 0 0 1rem;
  color: rgba(241, 245, 249, 0.96);
  font-size: 2.2rem;
  line-height: 1.08;
  text-wrap: balance;
}

.network-hero-copy :deep(p) {
  margin: 0.35rem 0 0;
  color: rgba(203, 213, 225, 0.8);
  font-size: 1.05rem;
  line-height: 1.35;
}

</style>
