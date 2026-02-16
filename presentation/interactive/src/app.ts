import { Application, Container, Graphics, Point, Text, TextStyle } from "pixi.js";
import { KeyController, type ControlAction } from "./input/keys";
import { createDefaultRuntimeStory } from "./model/story";
import type { LayoutMetrics } from "./model/types";
import { DecodeFxLayer } from "./render/decode-fx";
import {
  BACKGROUND_COLOR,
  FAILURE_ACCENT,
  LEFT_ACCENT,
  SUCCESS_ACCENT,
  WIRE_ACCENT,
  computeLayout,
} from "./render/layout";
import { PacketLayer } from "./render/packets";
import { SchemaPanel } from "./render/panels";
import { colorForDisplayType } from "./render/type-colors";
import { packetRows } from "./sim/packet";
import { WireEngine, type EngineConfig } from "./sim/engine";
import { getNextStateId, getPreviousStateId } from "./sim/transitions";

interface SlotHighlight {
  path: string;
  color: number;
  ttlSec: number;
}

const debugStyle = new TextStyle({
  fill: 0x8ea5c0,
  fontFamily: "Menlo, monospace",
  fontSize: 14,
});

const pauseButtonStyle = new TextStyle({
  fill: 0xe8edf7,
  fontFamily: "Menlo, monospace",
  fontSize: 14,
  fontWeight: "700",
});

const FAILURE_FOCUS_SEC = 0.3;
const FOCUS_DIM_ALPHA = 0.4;

const baseEngineConfig = (layout: LayoutMetrics): EngineConfig => {
  return {
    emitIntervalSec: 1.22,
    packetSpeedPxPerSec: 180,
    spawnX: layout.wireStartX + 116,
    decodeX: layout.decodeX,
    despawnX: layout.wireEndX + 120,
    packetY: layout.packetY,
    initialPacketCount: 3,
    initialPacketSpacing: 220,
  };
};

const leafPath = (path: string): string => {
  const parts = path.split(".");
  return parts[parts.length - 1] ?? path;
};

export const startInteractiveApp = async (host: HTMLElement): Promise<() => void> => {
  const app = new Application({
    backgroundColor: BACKGROUND_COLOR,
    antialias: true,
    resizeTo: window,
  });
  host.appendChild(app.view as HTMLCanvasElement);

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
  let pathColorByPath = new Map<string, number>();
  const colorForPath = (path: string): number => {
    return pathColorByPath.get(path) ?? WIRE_ACCENT;
  };

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

  let layout = computeLayout(app.renderer.width, app.renderer.height);
  let leftPanel = new SchemaPanel(
    "Writer",
    layout.panelWidth,
    layout.panelHeight,
    LEFT_ACCENT,
    false,
  );
  let rightPanel = new SchemaPanel(
    "Reader",
    layout.panelWidth,
    layout.panelHeight,
    0x8b5cf6,
    true,
  );
  panelLayer.addChild(leftPanel, rightPanel);

  const engine = new WireEngine(story, baseEngineConfig(layout));
  let currentStateId = engine.stateId();
  let slotHighlights: SlotHighlight[] = [];
  let laneCentersByPath = new Map<string, number>();
  let activePaths = new Set<string>();
  let failureFocusPath: string | null = null;
  let failureFocusTtlSec = 0;
  let wireTopY = layout.wireY - layout.wireHeight / 2;
  let wireBottomY = layout.wireY + layout.wireHeight / 2;
  let wireCenterY = layout.wireY;
  let debugVisible = false;

  const placePanels = (): void => {
    leftPanel.position.set(layout.leftPanelX - layout.panelWidth / 2, layout.panelY - layout.panelHeight / 2);
    rightPanel.position.set(layout.rightPanelX - layout.panelWidth / 2, layout.panelY - layout.panelHeight / 2);
  };

  const redrawPauseButton = (): void => {
    const width = 118;
    const height = 38;
    pauseButtonBg.clear();
    pauseButtonBg.lineStyle(1.8, 0x4b607f, 0.95);
    pauseButtonBg.beginFill(0x111b2b, 0.95);
    pauseButtonBg.drawRoundedRect(0, 0, width, height, 9);
    pauseButtonBg.endFill();

    pauseButtonText.text = engine.isPaused() ? "Resume" : "Pause";
    pauseButtonText.x = (width - pauseButtonText.width) / 2;
    pauseButtonText.y = (height - pauseButtonText.height) / 2;
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
      const color = colorForPath(path);
      const laneAlpha = failureFocusPath !== null && path !== failureFocusPath ? FOCUS_DIM_ALPHA : 1;
      wireGraphics.lineStyle(2.0, color, 0.42 * laneAlpha);
      wireGraphics.moveTo(layout.wireStartX + 10, laneY);
      wireGraphics.lineTo(layout.wireEndX - 10, laneY);

      wireGraphics.beginFill(color, 0.6 * laneAlpha);
      wireGraphics.drawCircle(layout.wireStartX + 8, laneY, 2.5);
      wireGraphics.drawCircle(layout.wireEndX - 8, laneY, 2.5);
      wireGraphics.endFill();
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
    const rightVersion = story.versions.get(state.rightVersionId);
    if (leftVersion === undefined || rightVersion === undefined) {
      throw new Error(`state '${state.id}' references unknown schema version`);
    }

    activePaths = new Set([
      ...leftVersion.fields.map((field) => field.path),
      ...rightVersion.fields.map((field) => field.path),
    ]);

    const nextColors = new Map<string, number>();
    const addColors = (fields: readonly { path: string; displayType: string }[]): void => {
      for (const field of fields) {
        if (!nextColors.has(field.path)) {
          nextColors.set(field.path, colorForDisplayType(field.displayType));
        }
      }
    };
    addColors(leftVersion.fields);
    addColors(rightVersion.fields);
    pathColorByPath = nextColors;

    leftPanel.setSchema(leftVersion.id, leftVersion.fields, lanePaths);
    rightPanel.setSchema(rightVersion.id, rightVersion.fields, lanePaths);
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
      LEFT_ACCENT,
      false,
    );
    rightPanel = new SchemaPanel(
      "Reader",
      layout.panelWidth,
      layout.panelHeight,
      0x8b5cf6,
      true,
    );
    panelLayer.addChild(leftPanel, rightPanel);
    placePanels();
  };

  const applyLayout = (): void => {
    layout = computeLayout(app.renderer.width, app.renderer.height);
    rebuildPanelsIfNeeded();
    engine.updateGeometry({
      spawnX: layout.wireStartX + 116,
      decodeX: layout.decodeX,
      despawnX: layout.wireEndX + 120,
      packetY: layout.packetY,
    });
    debugText.x = layout.width - 315;
    debugText.y = 14;
    renderSchemas();
  };

  const laneCenterYForPath = (path: string): number => {
    return laneCentersByPath.get(path) ?? wireCenterY;
  };

  const seedHighlights = (
    eventPacketId: number,
    matchedPaths: readonly string[],
    failingPath: string | undefined,
  ): void => {
    for (const path of matchedPaths) {
      const to = rightPanel.slotGlobalEntry(path) ?? rightPanel.slotGlobalCenter(path);
      if (to !== null) {
        const from = packetLayer.rowGlobalAnchor(eventPacketId, path) ?? new Point(layout.decodeX - 18, to.y);
        fxLayer.addTransferChip({
          from,
          to,
          label: leafPath(path),
          color: SUCCESS_ACCENT,
          ttlSec: 0.42,
        });
      }
      slotHighlights.push({ path, color: SUCCESS_ACCENT, ttlSec: 0.4 });
    }

    if (failingPath !== undefined) {
      const failTo = rightPanel.slotGlobalEntry(failingPath) ?? rightPanel.slotGlobalCenter(failingPath);
      if (failTo !== null) {
        const failFrom = packetLayer.rowGlobalAnchor(eventPacketId, failingPath)
          ?? new Point(layout.decodeX - 18, laneCenterYForPath(failingPath));
        fxLayer.addTransferChip({
          from: failFrom,
          to: failTo,
          label: leafPath(failingPath),
          color: FAILURE_ACCENT,
          ttlSec: 0.48,
        });
      }
      const failCenter = rightPanel.slotGlobalCenter(failingPath);
      if (failCenter !== null) {
        fxLayer.addFailMark(failCenter, 0.52);
      }
      slotHighlights.push({ path: failingPath, color: FAILURE_ACCENT, ttlSec: 0.55 });
    }

    if (failingPath === undefined && matchedPaths.length === 0) {
      const fallback = new Point(layout.decodeX, layout.wireY);
      fxLayer.addFailMark(fallback, 0.4);
    }
  };

  const refreshHighlights = (deltaSec: number): void => {
    const next: SlotHighlight[] = [];
    const byPath = new Map<string, number>();

    for (const highlight of slotHighlights) {
      const ttl = highlight.ttlSec - deltaSec;
      if (ttl <= 0) {
        continue;
      }
      const updated: SlotHighlight = { ...highlight, ttlSec: ttl };
      next.push(updated);
      const previous = byPath.get(updated.path);
      if (previous === undefined || updated.color === FAILURE_ACCENT) {
        byPath.set(updated.path, updated.color);
      }
    }

    slotHighlights = next;
    rightPanel.clearHighlights();
    for (const [path, color] of byPath.entries()) {
      rightPanel.highlightSlot(path, color, color === FAILURE_ACCENT ? 3.1 : 2.6);
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
    debugText.text = `state=${state.id} left=${state.leftVersionId} right=${state.rightVersionId} paused=${engine.isPaused()}`;
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
        break;
      }
      case "reset": {
        engine.reset();
        renderSchemas();
        currentStateId = engine.stateId();
        fxLayer.clearAll();
        slotHighlights = [];
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
    versionLabel: string;
  }[] => {
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
        versionLabel: version.id,
      };
    });
  };

  const keyController = new KeyController(handleAction);

  window.addEventListener("resize", applyLayout);
  applyLayout();

  app.ticker.add(() => {
    const deltaSec = app.ticker.deltaMS / 1000;
    engine.step(deltaSec);

    const decodeEvents = engine.drainDecodeEvents();
    for (const decodeEvent of decodeEvents) {
      seedHighlights(decodeEvent.packetId, decodeEvent.matchedPaths, decodeEvent.result.failingPath);
    }

    packetLayer.syncPackets(toPacketViews());

    refreshHighlights(deltaSec);
    fxLayer.tick(deltaSec);

    if (currentStateId !== engine.stateId()) {
      currentStateId = engine.stateId();
      renderSchemas();
    }

    updateDebug();
  });

  return () => {
    keyController.dispose();
    window.removeEventListener("resize", applyLayout);
    app.destroy(true);
  };
};
