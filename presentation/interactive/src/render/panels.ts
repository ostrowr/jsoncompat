import { Container, Graphics, Point, Text, TextStyle } from "pixi.js";
import type { FlattenedField } from "../model/types";
import { colorForDisplayType } from "./type-colors";

interface SlotView {
  path: string;
  versionLabel: string;
  y: number;
  centerY: number;
  width: number;
  height: number;
  content: Container;
  graphics: Graphics;
  baseColor: number;
}

interface SchemaColumnSpec {
  versionLabel: string;
  fields: readonly FlattenedField[];
}

const titleStyle = new TextStyle({
  fill: 0xe8edf7,
  fontFamily: "Georgia, serif",
  fontSize: 34,
  fontWeight: "500",
  dropShadow: true,
  dropShadowAlpha: 0.35,
  dropShadowBlur: 2,
  dropShadowDistance: 1,
  dropShadowColor: 0x000000,
});

const versionStyle = new TextStyle({
  fill: 0x9fb5cd,
  fontFamily: "Menlo, monospace",
  fontSize: 20,
  fontWeight: "500",
});

const columnVersionStyle = new TextStyle({
  fill: 0x88a3c2,
  fontFamily: "Menlo, monospace",
  fontSize: 14,
  fontWeight: "700",
});

const PANEL_BG = 0x0f1726;
const CHIP_BG = 0x111b2b;
const CHIP_BORDER = 0x5a7196;

const fitPathLabel = (path: string, style: TextStyle, maxWidth: number): string => {
  const baseSuffix = ": ";
  const measure = new Text("", style);
  const fullLabel = `${path}${baseSuffix}`;
  measure.text = fullLabel;
  if (measure.width <= maxWidth) {
    measure.destroy();
    return fullLabel;
  }

  const minLabel = `...${baseSuffix}`;
  measure.text = minLabel;
  if (measure.width > maxWidth) {
    measure.destroy();
    return baseSuffix;
  }

  let prefix = path;
  while (prefix.length > 0) {
    const candidate = `${prefix}...${baseSuffix}`;
    measure.text = candidate;
    if (measure.width <= maxWidth) {
      measure.destroy();
      return candidate;
    }
    prefix = prefix.slice(0, -1);
  }

  measure.destroy();
  return minLabel;
};

export class SchemaPanel extends Container {
  private readonly widthPx: number;
  private readonly heightPx: number;
  private readonly accent: number;
  private readonly panelGraphics: Graphics;
  private readonly titleText: Text;
  private readonly versionText: Text;
  private readonly slotsLayer: Container;
  private slotsByVersionAndPath: Map<string, SlotView> = new Map();
  private slotsByPath: Map<string, SlotView[]> = new Map();
  private laneCentersByPath: Map<string, number> = new Map();
  private focusPath: string | null = null;
  private dimAlpha = 0.4;
  private schemaTransitionProgress = 1;

  public constructor(
    title: string,
    widthPx: number,
    heightPx: number,
    accent: number,
  ) {
    super();
    this.widthPx = widthPx;
    this.heightPx = heightPx;
    this.accent = accent;

    this.panelGraphics = new Graphics();
    this.titleText = new Text(title, titleStyle);
    this.versionText = new Text("", versionStyle);
    this.slotsLayer = new Container();

    this.addChild(this.panelGraphics, this.titleText, this.versionText, this.slotsLayer);
    this.drawPanel();
  }

  public setSchema(versionLabel: string, fields: readonly FlattenedField[], laneOrder?: readonly string[]): void {
    this.setSchemaUnion([{ versionLabel, fields }], laneOrder);
  }

  public setSchemaUnion(columns: readonly SchemaColumnSpec[], laneOrder?: readonly string[]): void {
    if (columns.length === 0) {
      throw new Error("setSchemaUnion requires at least one reader version");
    }

    this.versionText.text = columns.map((column) => column.versionLabel).join(" | ");
    this.layoutHeader();
    this.renderSlots(columns, laneOrder);
    this.schemaTransitionProgress = 0;
    this.applySchemaTransition();
  }

  public tick(deltaSec: number): void {
    if (this.schemaTransitionProgress >= 1) {
      return;
    }
    this.schemaTransitionProgress = Math.min(1, this.schemaTransitionProgress + deltaSec / 0.52);
    this.applySchemaTransition();
  }

  public clearHighlights(): void {
    for (const slot of this.slotsByVersionAndPath.values()) {
      this.drawSlot(slot, slot.baseColor, 1.6);
    }
  }

  public highlightSlot(path: string, color: number, strokeWidth = 2.8, versionLabel?: string): void {
    if (versionLabel !== undefined) {
      const slot = this.slotsByVersionAndPath.get(this.slotKey(versionLabel, path));
      if (slot !== undefined) {
        this.drawSlot(slot, color, strokeWidth);
      }
      return;
    }

    const slots = this.slotsByPath.get(path);
    if (slots === undefined) {
      return;
    }
    for (const slot of slots) {
      this.drawSlot(slot, color, strokeWidth);
    }
  }

  public setDimFocus(path: string | null, dimAlpha: number): void {
    this.focusPath = path;
    this.dimAlpha = dimAlpha;
    this.applyDimFocus();
  }

  public slotGlobalCenter(path: string): Point | null {
    const slot = this.firstSlotForPath(path);
    if (slot === null) {
      return null;
    }
    const global = this.toGlobal(new Point(slot.graphics.x + slot.width / 2, slot.centerY));
    return new Point(global.x, global.y);
  }

  public slotGlobalEntry(path: string): Point | null {
    const slot = this.firstSlotForPath(path);
    if (slot === null) {
      return null;
    }
    const global = this.toGlobal(new Point(slot.graphics.x + 10, slot.centerY));
    return new Point(global.x, global.y);
  }

  public laneGlobalCenter(path: string): Point | null {
    const laneCenter = this.laneCentersByPath.get(path);
    if (laneCenter === undefined) {
      return null;
    }
    const global = this.toGlobal(new Point(this.widthPx / 2, laneCenter));
    return new Point(global.x, global.y);
  }

  private slotKey(versionLabel: string, path: string): string {
    return `${versionLabel}::${path}`;
  }

  private firstSlotForPath(path: string): SlotView | null {
    const slots = this.slotsByPath.get(path);
    return slots?.[0] ?? null;
  }

  private drawPanel(): void {
    this.panelGraphics.clear();
    this.panelGraphics.lineStyle(2, this.accent, 0.82);
    this.panelGraphics.beginFill(PANEL_BG, 0.97);
    this.panelGraphics.drawRoundedRect(0, 0, this.widthPx, this.heightPx, 16);
    this.panelGraphics.endFill();
    this.panelGraphics.lineStyle(1, 0x1d2a3e, 0.72);
    this.panelGraphics.drawRoundedRect(6, 6, this.widthPx - 12, this.heightPx - 12, 13);

    this.layoutHeader();
  }

  private layoutHeader(): void {
    this.titleText.x = (this.widthPx - this.titleText.width) / 2;
    this.titleText.y = 18;
    this.versionText.x = (this.widthPx - this.versionText.width) / 2;
    this.versionText.y = 66;
  }

  private renderSlots(columns: readonly SchemaColumnSpec[], laneOrder?: readonly string[]): void {
    this.slotsLayer.removeChildren();
    this.slotsByVersionAndPath = new Map();
    this.slotsByPath = new Map();
    this.laneCentersByPath = new Map();

    const unionFieldPaths = new Set<string>();
    for (const column of columns) {
      for (const field of column.fields) {
        unionFieldPaths.add(field.path);
      }
    }

    const lanes = (laneOrder !== undefined && laneOrder.length > 0)
      ? laneOrder
      : Array.from(unionFieldPaths.values());
    const laneCount = Math.max(1, lanes.length);

    const startY = 134;
    const bottomPadding = 34;
    const availableHeight = Math.max(72, this.heightPx - startY - bottomPadding);
    const lanePitch = availableHeight / laneCount;
    const laneGap = laneCount >= 4 ? 13 : 9;
    const chipHeight = Math.max(30, Math.min(48, lanePitch - laneGap));

    const laneIndexByPath = new Map<string, number>();
    lanes.forEach((path, index) => {
      laneIndexByPath.set(path, index);
      const centerY = startY + lanePitch * index + lanePitch / 2;
      this.laneCentersByPath.set(path, centerY);
    });

    const columnCount = columns.length;
    const slotAreaX = 29;
    const slotAreaWidth = this.widthPx - 58;
    const columnGap = columnCount > 1 ? 14 : 0;
    const columnWidth = (slotAreaWidth - columnGap * (columnCount - 1)) / columnCount;
    const compactMode = columnCount > 1;
    const rowFontSize = compactMode ? 16 : 22;

    for (let columnIndex = 0; columnIndex < columns.length; columnIndex += 1) {
      const column = columns[columnIndex];
      if (column === undefined) {
        continue;
      }

      const columnX = slotAreaX + columnIndex * (columnWidth + columnGap);

      if (compactMode) {
        const columnVersionText = new Text(column.versionLabel, columnVersionStyle);
        columnVersionText.x = columnX + (columnWidth - columnVersionText.width) / 2;
        columnVersionText.y = startY - 26;
        this.slotsLayer.addChild(columnVersionText);
      }

      for (let fieldIndex = 0; fieldIndex < column.fields.length; fieldIndex += 1) {
        const field = column.fields[fieldIndex];
        if (field === undefined) {
          continue;
        }

        const laneIndex = laneIndexByPath.get(field.path) ?? fieldIndex;
        const centerY = startY + lanePitch * laneIndex + lanePitch / 2;
        const y = centerY - chipHeight / 2;
        const slotContent = new Container();
        const graphics = new Graphics();
        graphics.x = columnX;

        const baseColor = CHIP_BORDER;
        const keyStyle = new TextStyle({
          fill: 0xe8edf7,
          fontFamily: "Menlo, monospace",
          fontSize: rowFontSize,
          fontWeight: "500",
          dropShadow: true,
          dropShadowAlpha: 0.25,
          dropShadowBlur: 1,
          dropShadowDistance: 1,
          dropShadowColor: 0x000000,
        });

        const typeStyle = new TextStyle({
          fontFamily: "Menlo, monospace",
          fontSize: rowFontSize,
          fontWeight: "600",
          fill: colorForDisplayType(field.displayType),
          dropShadow: true,
          dropShadowAlpha: 0.25,
          dropShadowBlur: 1,
          dropShadowDistance: 1,
          dropShadowColor: 0x000000,
        });

        const optionalStyle = new TextStyle({
          fill: 0xe8edf7,
          fontFamily: "Menlo, monospace",
          fontSize: rowFontSize - 1,
          fontWeight: "500",
          dropShadow: true,
          dropShadowAlpha: 0.25,
          dropShadowBlur: 1,
          dropShadowDistance: 1,
          dropShadowColor: 0x000000,
        });

        const typeText = new Text(field.displayType, typeStyle);
        const optionalText = field.required ? null : new Text(" (optional)", optionalStyle);
        const optionalWidth = optionalText?.width ?? 0;
        const maxKeyWidth = Math.max(18, columnWidth - 14 - typeText.width - optionalWidth);
        const keyText = new Text(fitPathLabel(field.path, keyStyle, maxKeyWidth), keyStyle);

        const totalWidth = keyText.width + typeText.width + optionalWidth;
        const startX = columnX + (columnWidth - totalWidth) / 2;
        keyText.x = startX;
        keyText.y = centerY - keyText.height / 2;
        typeText.x = keyText.x + keyText.width;
        typeText.y = centerY - typeText.height / 2;

        if (optionalText !== null) {
          optionalText.x = typeText.x + typeText.width;
          optionalText.y = centerY - optionalText.height / 2;
        }

        const slot: SlotView = {
          path: field.path,
          versionLabel: column.versionLabel,
          y,
          centerY,
          width: columnWidth,
          height: chipHeight,
          content: slotContent,
          graphics,
          baseColor,
        };

        this.drawSlot(slot, baseColor, 1.6);

        slotContent.addChild(graphics, keyText, typeText);
        if (optionalText !== null) {
          slotContent.addChild(optionalText);
        }
        this.slotsLayer.addChild(slotContent);

        this.slotsByVersionAndPath.set(this.slotKey(column.versionLabel, field.path), slot);
        const slots = this.slotsByPath.get(field.path);
        if (slots === undefined) {
          this.slotsByPath.set(field.path, [slot]);
        } else {
          slots.push(slot);
        }
      }
    }

    this.applyDimFocus();
  }

  private drawSlot(slot: SlotView, strokeColor: number, strokeWidth: number): void {
    slot.graphics.clear();
    slot.graphics.lineStyle(strokeWidth, strokeColor, 0.98);
    slot.graphics.beginFill(CHIP_BG, 0.97);
    slot.graphics.drawRoundedRect(0, slot.y, slot.width, slot.height, 7);
    slot.graphics.endFill();
  }

  private applyDimFocus(): void {
    for (const slot of this.slotsByVersionAndPath.values()) {
      if (this.focusPath === null || slot.path === this.focusPath) {
        slot.content.alpha = 1;
      } else {
        slot.content.alpha = this.dimAlpha;
      }
    }
  }

  private applySchemaTransition(): void {
    const t = this.schemaTransitionProgress;
    const eased = 1 - Math.pow(1 - t, 3);
    this.versionText.alpha = 0.52 + eased * 0.48;
    this.versionText.scale.set(0.94 + eased * 0.06);
    this.versionText.x = (this.widthPx - this.versionText.width) / 2;
    this.versionText.y = 66 - (1 - eased) * 4;
    this.slotsLayer.alpha = 0.72 + eased * 0.28;
  }
}
