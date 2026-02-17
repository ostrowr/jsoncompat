import { Application, Container, Graphics, Point, Text, TextStyle } from "pixi.js";
import { KeyController, type ControlAction } from "./input/keys";
import { createDefaultRuntimeStory } from "./model/story";
import type { LayoutMetrics } from "./model/types";
import { DecodeFxLayer } from "./render/decode-fx";
import {
  BACKGROUND_COLOR,
  FAILURE_ACCENT,
  SUCCESS_ACCENT,
  WIRE_ACCENT,
  computeLayout,
} from "./render/layout";
import { PacketLayer } from "./render/packets";
import { SchemaPanel } from "./render/panels";
import { packetRows } from "./sim/packet";
import { WireEngine, type EngineConfig } from "./sim/engine";
import { getNextStateId, getPreviousStateId } from "./sim/transitions";

interface SlotHighlight {
  path: string;
  versionLabel: string | undefined;
  color: number;
  ttlSec: number;
  delaySec: number;
}

const debugStyle = new TextStyle({
  fill: 0x8ea5c0,
  fontFamily: "Menlo, monospace",
  fontSize: 14,
});

const pauseButtonStyle = new TextStyle({
  fill: 0xe8edf7,
  fontFamily: "Menlo, monospace",
  fontSize: 16,
  fontWeight: "700",
});

const stateChipStyle = new TextStyle({
  fill: 0xe8edf7,
  fontFamily: "Menlo, monospace",
  fontSize: 18,
  fontWeight: "700",
});

const GATE_FLASH_SEC = 0.5;
const SLOT_FLASH_DELAY_SEC = 0.12;
const SLOT_FLASH_SUCCESS_SEC = 0.42;
const SLOT_FLASH_FAILURE_SEC = 0.58;
const PACKET_SPAWN_OFFSET_PX = 72;
const PACKET_FADE_IN_DISTANCE_PX = 46;
const PACKET_FADE_OUT_LEAD_PX = 8;
const PACKET_FADE_OUT_DISTANCE_PX = 56;

const clamp01 = (value: number): number => {
  return Math.max(0, Math.min(1, value));
};

const smoothStep01 = (value: number): number => {
  const t = clamp01(value);
  return t * t * (3 - 2 * t);
};

const baseEngineConfig = (layout: LayoutMetrics): EngineConfig => {
  return {
    emitIntervalSec: 1.6,
    packetSpeedPxPerSec: 148,
    spawnX: layout.wireStartX + PACKET_SPAWN_OFFSET_PX,
    decodeX: layout.decodeX,
    despawnX: layout.wireEndX + 120,
    packetY: layout.packetY,
    initialPacketCount: 2,
    initialPacketSpacing: 264,
  };
};

export const startInteractiveApp = async (host: HTMLElement): Promise<() => void> => {
  host.style.position = "relative";

  const app = new Application({
    backgroundColor: BACKGROUND_COLOR,
    antialias: true,
    resizeTo: window,
  });
  host.appendChild(app.view as HTMLCanvasElement);

  const emissionControl = document.createElement("div");
  emissionControl.style.position = "absolute";
  emissionControl.style.left = "18px";
  emissionControl.style.top = "16px";
  emissionControl.style.padding = "9px 11px";
  emissionControl.style.border = "1.5px solid #4f6790";
  emissionControl.style.borderRadius = "10px";
  emissionControl.style.background = "rgba(17, 27, 43, 0.9)";
  emissionControl.style.color = "#e8edf7";
  emissionControl.style.fontFamily = "Menlo, monospace";
  emissionControl.style.fontSize = "13px";
  emissionControl.style.zIndex = "5";

  const emissionTitle = document.createElement("div");
  emissionTitle.textContent = "Emission Rate";
  emissionTitle.style.fontWeight = "700";
  emissionTitle.style.marginBottom = "5px";

  const emissionSlider = document.createElement("input");
  emissionSlider.type = "range";
  emissionSlider.min = "0.4";
  emissionSlider.max = "2.4";
  emissionSlider.step = "0.05";
  emissionSlider.style.width = "190px";

  const emissionValue = document.createElement("div");
  emissionValue.style.marginTop = "5px";
  emissionValue.style.color = "#9fb5cd";

  emissionControl.append(emissionTitle, emissionSlider, emissionValue);
  host.appendChild(emissionControl);

  const story = createDefaultRuntimeStory();
  const lanePaths: string[] = [];
  const lanePathSet = new Set<string>();
  for (const version of story.versions.values()) {
    for (const field of version.fields) {
      if (!lanePathSet.has(field.path)) {
        lanePathSet.add(field.path);
        lanePaths.push(field.path);
      }
    }
  }
  const rootLayer = new Container();
  const wireLayer = new Container();
  const panelLayer = new Container();
  const packetLayer = new PacketLayer();
  const fxLayer = new DecodeFxLayer();
  const uiLayer = new Container();
  rootLayer.addChild(wireLayer, panelLayer, packetLayer, fxLayer, uiLayer);
  app.stage.addChild(rootLayer);

  const wireGraphics = new Graphics();
  wireLayer.addChild(wireGraphics);

  const debugText = new Text("", debugStyle);
  debugText.visible = false;
  uiLayer.addChild(debugText);

  const pauseButton = new Container();
  const pauseButtonBg = new Graphics();
  const pauseButtonText = new Text("Pause", pauseButtonStyle);
  pauseButton.addChild(pauseButtonBg, pauseButtonText);
  pauseButton.eventMode = "static";
  pauseButton.cursor = "pointer";
  uiLayer.addChild(pauseButton);

  const stateChip = new Container();
  const stateChipBg = new Graphics();
  const stateChipText = new Text("", stateChipStyle);
  stateChip.addChild(stateChipBg, stateChipText);
  uiLayer.addChild(stateChip);

  let layout = computeLayout(app.renderer.width, app.renderer.height);
  let leftPanel = new SchemaPanel(
    "Writer",
    layout.panelWidth,
    layout.panelHeight,
    WIRE_ACCENT,
  );
  let rightPanel = new SchemaPanel(
    "Reader",
    layout.panelWidth,
    layout.panelHeight,
    WIRE_ACCENT,
  );
  panelLayer.addChild(leftPanel, rightPanel);

  const engine = new WireEngine(story, baseEngineConfig(layout));

  const formatEmitRate = (ratePerSec: number): string => `${ratePerSec.toFixed(2)} msg/s`;
  const applyEmissionSlider = (): void => {
    const rate = Math.max(0.2, Number(emissionSlider.value));
    engine.setEmitIntervalSec(1 / rate);
    emissionValue.textContent = formatEmitRate(rate);
  };
  const onEmissionInput = (): void => {
    applyEmissionSlider();
  };
  const defaultRate = 1 / engine.emitIntervalSec();
  emissionSlider.value = defaultRate.toFixed(2);
  emissionValue.textContent = formatEmitRate(defaultRate);
  emissionSlider.addEventListener("input", onEmissionInput);

  let currentStateId = engine.stateId();
  let slotHighlights: SlotHighlight[] = [];
  let laneCentersByPath = new Map<string, number>();
  let activePaths = new Set<string>();
  let wireTopY = layout.wireY - layout.wireHeight / 2;
  let wireBottomY = layout.wireY + layout.wireHeight / 2;
  let wireCenterY = layout.wireY;
  let debugVisible = false;
  let gateFlashColor = SUCCESS_ACCENT;
  let gateFlashTtlSec = 0;
  let schemaChipTransitionSec = 0;

  const placePanels = (): void => {
    leftPanel.position.set(layout.leftPanelX - layout.panelWidth / 2, layout.panelY - layout.panelHeight / 2);
    rightPanel.position.set(layout.rightPanelX - layout.panelWidth / 2, layout.panelY - layout.panelHeight / 2);
  };

  const redrawPauseButton = (): void => {
    const width = 126;
    const height = 42;
    pauseButtonBg.clear();
    pauseButtonBg.lineStyle(2, 0x587198, 0.96);
    pauseButtonBg.beginFill(0x111b2b, 0.95);
    pauseButtonBg.drawRoundedRect(0, 0, width, height, 10);
    pauseButtonBg.endFill();

    pauseButtonText.text = engine.isPaused() ? "Resume" : "Pause";
    pauseButtonText.x = (width - pauseButtonText.width) / 2;
    pauseButtonText.y = (height - pauseButtonText.height) / 2;
    pauseButton.position.set(layout.width - width - 18, 16);
  };

  const redrawStateChip = (leftVersionId: string, rightVersionLabel: string): void => {
    stateChipText.text = `Writer ${leftVersionId} -> Reader ${rightVersionLabel}`;
    const width = stateChipText.width + 28;
    const height = 34;
    stateChipBg.clear();
    stateChipBg.lineStyle(1.8, WIRE_ACCENT, 0.94);
    stateChipBg.beginFill(0x111b2b, 0.94);
    stateChipBg.drawRoundedRect(0, 0, width, height, 11);
    stateChipBg.endFill();
    stateChipText.x = 14;
    stateChipText.y = (height - stateChipText.height) / 2;
    stateChip.position.set((layout.width - width) / 2, 18);
    schemaChipTransitionSec = 0.52;
  };

  const tickStateChip = (deltaSec: number): void => {
    if (schemaChipTransitionSec <= 0) {
      stateChip.alpha = 1;
      stateChip.scale.set(1);
      return;
    }
    const durationSec = 0.52;
    schemaChipTransitionSec = Math.max(0, schemaChipTransitionSec - deltaSec);
    const t = 1 - schemaChipTransitionSec / durationSec;
    const eased = 1 - Math.pow(1 - t, 3);
    stateChip.alpha = 0.74 + eased * 0.26;
    const scale = 0.97 + eased * 0.03;
    stateChip.scale.set(scale);
    const width = stateChipBg.width;
    stateChip.x = (layout.width - width * scale) / 2;
  };

  const drawWire = (): void => {
    const laneCenters = Array.from(laneCentersByPath.values());
    if (laneCenters.length > 0) {
      const minLaneY = Math.min(...laneCenters);
      const maxLaneY = Math.max(...laneCenters);
      const paddedTop = minLaneY - 34;
      const paddedBottom = maxLaneY + 34;
      const dynamicHeight = Math.max(layout.wireHeight, paddedBottom - paddedTop);
      wireCenterY = (paddedTop + paddedBottom) / 2;
      wireTopY = wireCenterY - dynamicHeight / 2;
      wireBottomY = wireCenterY + dynamicHeight / 2;
    } else {
      wireTopY = layout.wireY - layout.wireHeight / 2;
      wireBottomY = layout.wireY + layout.wireHeight / 2;
      wireCenterY = layout.wireY;
    }
    const y = wireTopY;
    const wireHeight = wireBottomY - wireTopY;

    wireGraphics.clear();

    const sortedLaneEntries = Array.from(laneCentersByPath.entries()).sort((a, b) => a[1] - b[1]);
    for (const [path, laneY] of sortedLaneEntries) {
      if (!activePaths.has(path)) {
        continue;
      }
      if (laneY < y + 10 || laneY > y + wireHeight - 10) {
        continue;
      }
      const color = 0x60728d;
      wireGraphics.lineStyle(2.8, color, 0.42);
      wireGraphics.moveTo(layout.wireStartX + 10, laneY);
      wireGraphics.lineTo(layout.wireEndX - 10, laneY);
    }

    const gateTop = layout.panelY - layout.panelHeight / 2 + 8;
    const gateBottom = gateTop + Math.max(32, layout.panelHeight - 16);
    const gateX = layout.decodeX;

    // Clean decode gate: a single vertical divider with a soft halo.
    wireGraphics.lineStyle(5, 0x24344d, 0.28);
    wireGraphics.moveTo(gateX, gateTop);
    wireGraphics.lineTo(gateX, gateBottom);
    wireGraphics.lineStyle(1.6, 0x6f86aa, 0.72);
    wireGraphics.moveTo(gateX, gateTop);
    wireGraphics.lineTo(gateX, gateBottom);

    if (gateFlashTtlSec > 0) {
      const alpha = Math.max(0.1, gateFlashTtlSec / GATE_FLASH_SEC) * 0.5;
      wireGraphics.lineStyle(7, gateFlashColor, alpha * 0.22);
      wireGraphics.moveTo(gateX, gateTop);
      wireGraphics.lineTo(gateX, gateBottom);
      wireGraphics.lineStyle(2.6, gateFlashColor, alpha * 0.95);
      wireGraphics.moveTo(gateX, gateTop);
      wireGraphics.lineTo(gateX, gateBottom);
    }
  };

  const refreshLaneCenters = (): void => {
    const next = new Map<string, number>();
    for (const path of lanePaths) {
      const leftLane = leftPanel.laneGlobalCenter(path);
      const rightLane = rightPanel.laneGlobalCenter(path);
      const laneY = leftLane?.y ?? rightLane?.y;
      if (laneY !== undefined) {
        next.set(path, laneY);
      }
    }
    laneCentersByPath = next;
    packetLayer.setLaneCenters(laneCentersByPath);
    drawWire();
  };

  const renderSchemas = (): void => {
    const state = story.states.get(engine.stateId());
    if (state === undefined) {
      throw new Error(`unknown runtime state '${engine.stateId()}'`);
    }

    const leftVersion = story.versions.get(state.leftVersionId);
    if (leftVersion === undefined) {
      throw new Error(`state '${state.id}' references unknown schema version`);
    }
    const rightVersions = state.rightVersionIds.map((rightVersionId) => {
      const rightVersion = story.versions.get(rightVersionId);
      if (rightVersion === undefined) {
        throw new Error(`state '${state.id}' references unknown right schema version '${rightVersionId}'`);
      }
      return rightVersion;
    });
    if (rightVersions.length === 0) {
      throw new Error(`state '${state.id}' has no right schema versions`);
    }

    activePaths = new Set([
      ...leftVersion.fields.map((field) => field.path),
      ...rightVersions.flatMap((version) => version.fields.map((field) => field.path)),
    ]);

    leftPanel.setSchema(leftVersion.id, leftVersion.fields, lanePaths);
    rightPanel.setSchemaUnion(rightVersions.map((version) => ({
      versionLabel: version.id,
      fields: version.fields,
    })), lanePaths);
    packetLayer.setDenseMode(activePaths.size >= 4);
    redrawStateChip(leftVersion.id, rightVersions.map((version) => version.id).join(" | "));
    refreshLaneCenters();
  };

  const rebuildPanelsIfNeeded = (): void => {
    const oldWidth = leftPanel.width;
    const oldHeight = leftPanel.height;
    const widthDelta = Math.abs(oldWidth - layout.panelWidth);
    const heightDelta = Math.abs(oldHeight - layout.panelHeight);
    if (widthDelta < 2 && heightDelta < 2) {
      placePanels();
      return;
    }

    panelLayer.removeChild(leftPanel, rightPanel);
    leftPanel.destroy({ children: true });
    rightPanel.destroy({ children: true });

    leftPanel = new SchemaPanel(
      "Writer",
      layout.panelWidth,
      layout.panelHeight,
      WIRE_ACCENT,
    );
    rightPanel = new SchemaPanel(
      "Reader",
      layout.panelWidth,
      layout.panelHeight,
      WIRE_ACCENT,
    );
    panelLayer.addChild(leftPanel, rightPanel);
    placePanels();
  };

  const applyLayout = (): void => {
    layout = computeLayout(app.renderer.width, app.renderer.height);
    rebuildPanelsIfNeeded();
    engine.updateGeometry({
      spawnX: layout.wireStartX + PACKET_SPAWN_OFFSET_PX,
      decodeX: layout.decodeX,
      despawnX: layout.wireEndX + 120,
      packetY: layout.packetY,
    });
    debugText.x = layout.width - 315;
    debugText.y = 14;
    redrawPauseButton();
    renderSchemas();
  };

  const seedHighlights = (
    matchedPaths: readonly string[],
    failingPath: string | undefined,
    matchedReaderVersionId?: string,
  ): void => {
    if (failingPath !== undefined) {
      gateFlashColor = FAILURE_ACCENT;
      gateFlashTtlSec = GATE_FLASH_SEC;
    } else {
      gateFlashColor = SUCCESS_ACCENT;
      gateFlashTtlSec = GATE_FLASH_SEC;
    }

    for (const path of matchedPaths) {
      slotHighlights.push({
        path,
        versionLabel: matchedReaderVersionId,
        color: SUCCESS_ACCENT,
        ttlSec: SLOT_FLASH_SUCCESS_SEC,
        delaySec: SLOT_FLASH_DELAY_SEC,
      });
    }

    if (failingPath !== undefined) {
      slotHighlights.push({
        path: failingPath,
        versionLabel: matchedReaderVersionId,
        color: FAILURE_ACCENT,
        ttlSec: SLOT_FLASH_FAILURE_SEC,
        delaySec: SLOT_FLASH_DELAY_SEC,
      });
    }
  };

  const refreshHighlights = (deltaSec: number): void => {
    const next: SlotHighlight[] = [];
    const bySlot = new Map<string, SlotHighlight>();

    for (const highlight of slotHighlights) {
      const delaySec = Math.max(0, highlight.delaySec - deltaSec);
      const ttl = highlight.ttlSec - deltaSec;
      if (ttl <= 0) {
        continue;
      }
      const updated: SlotHighlight = { ...highlight, ttlSec: ttl, delaySec };
      next.push(updated);
      if (delaySec > 0) {
        continue;
      }
      const slotKey = `${updated.versionLabel ?? "*"}::${updated.path}`;
      const previous = bySlot.get(slotKey);
      if (previous === undefined || updated.color === FAILURE_ACCENT) {
        bySlot.set(slotKey, updated);
      }
    }

    slotHighlights = next;
    rightPanel.clearHighlights();
    for (const highlight of bySlot.values()) {
      rightPanel.highlightSlot(
        highlight.path,
        highlight.color,
        highlight.color === FAILURE_ACCENT ? 3.1 : 2.6,
        highlight.versionLabel,
      );
    }
  };

  const updateDebug = (): void => {
    if (!debugVisible) {
      debugText.visible = false;
      return;
    }

    const state = story.states.get(engine.stateId());
    if (state === undefined) {
      return;
    }

    debugText.visible = true;
    debugText.text = `state=${state.id} left=${state.leftVersionId} right=${state.rightVersionIds.join("|")} paused=${engine.isPaused()}`;
  };

  const transitionTo = (stateId: string): void => {
    if (stateId === engine.stateId()) {
      return;
    }
    engine.transitionTo(stateId);
    renderSchemas();
    currentStateId = stateId;
  };

  const handleAction = (action: ControlAction): void => {
    switch (action) {
      case "next": {
        transitionTo(getNextStateId(story, engine.stateId()));
        break;
      }
      case "prev": {
        transitionTo(getPreviousStateId(story, engine.stateId()));
        break;
      }
      case "pause": {
        engine.togglePaused();
        redrawPauseButton();
        break;
      }
      case "reset": {
        engine.reset();
        renderSchemas();
        currentStateId = engine.stateId();
        fxLayer.clearAll();
        slotHighlights = [];
        gateFlashTtlSec = 0;
        redrawPauseButton();
        break;
      }
      case "toggle_debug": {
        debugVisible = !debugVisible;
        break;
      }
      default:
        break;
    }
  };

  const toPacketViews = (): readonly {
    id: number;
    x: number;
    y: number;
    rows: readonly { path: string; keyText: string; valueText: string; displayType: string }[];
    color: number;
    alpha: number;
    versionLabel: string;
  }[] => {
    const packetAlphaAtX = (x: number): number => {
      const spawnX = layout.wireStartX + PACKET_SPAWN_OFFSET_PX;
      const fadeIn = smoothStep01((x - spawnX) / PACKET_FADE_IN_DISTANCE_PX);
      const fadeOutStart = layout.decodeX - PACKET_FADE_OUT_LEAD_PX;
      const fadeOut = 1 - smoothStep01((x - fadeOutStart) / PACKET_FADE_OUT_DISTANCE_PX);
      return clamp01(Math.min(fadeIn, fadeOut));
    };

    return engine.activePackets().map((packet) => {
      const version = story.versions.get(packet.schemaVersionId);
      if (version === undefined) {
        throw new Error(`packet references unknown schema version '${packet.schemaVersionId}'`);
      }
      return {
        id: packet.id,
        x: packet.x,
        y: packet.y,
        rows: packetRows(packet, version),
        color: 0,
        alpha: packetAlphaAtX(packet.x),
        versionLabel: version.id,
      };
    });
  };

  const keyController = new KeyController(handleAction);
  pauseButton.on("pointertap", () => {
    engine.togglePaused();
    redrawPauseButton();
  });

  window.addEventListener("resize", applyLayout);
  applyLayout();

  app.ticker.add(() => {
    const deltaSec = app.ticker.deltaMS / 1000;
    engine.step(deltaSec);

    const decodeEvents = engine.drainDecodeEvents();
    for (const decodeEvent of decodeEvents) {
      seedHighlights(
        decodeEvent.matchedPaths,
        decodeEvent.result.failingPath,
        decodeEvent.matchedReaderVersionId,
      );
    }

    if (gateFlashTtlSec > 0) {
      gateFlashTtlSec = Math.max(0, gateFlashTtlSec - deltaSec);
    }

    leftPanel.setDimFocus(null, 1);
    rightPanel.setDimFocus(null, 1);
    packetLayer.setDimFocus(null, 1);

    packetLayer.syncPackets(toPacketViews());

    refreshHighlights(deltaSec);
    fxLayer.tick(deltaSec);
    leftPanel.tick(deltaSec);
    rightPanel.tick(deltaSec);
    tickStateChip(deltaSec);
    drawWire();

    if (currentStateId !== engine.stateId()) {
      currentStateId = engine.stateId();
      renderSchemas();
    }

    updateDebug();
  });

  return () => {
    keyController.dispose();
    window.removeEventListener("resize", applyLayout);
    emissionSlider.removeEventListener("input", onEmissionInput);
    if (emissionControl.parentElement === host) {
      host.removeChild(emissionControl);
    }
    app.destroy(true);
  };
};
