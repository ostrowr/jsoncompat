import { Container, Graphics, Point, Text, TextStyle } from "pixi.js-legacy";
import type { PacketViewModel } from "../model/types";
import { colorForDisplayType } from "./type-colors";

interface RowChipSprite {
  path: string;
  textValue: string;
  rowBg: Graphics;
  keyLabel: Text;
  valueLabel: Text;
  rowWidth: number;
  rowHeight: number;
  centerY: number;
  anchorX: number;
}

interface PacketSprite {
  root: Container;
  envelope: Graphics;
  versionTag: Text;
  rowsByPath: Map<string, RowChipSprite>;
  rowSignature: string;
  versionLabel: string;
  lastTouchedFrame: number;
}

const versionTagStyle = new TextStyle({
  fill: 0xc8d4e8,
  fontFamily: "Menlo, monospace",
  fontSize: 13,
  fontWeight: "700",
  dropShadow: true,
  dropShadowAlpha: 0.25,
  dropShadowBlur: 1,
  dropShadowDistance: 1,
  dropShadowColor: 0x000000,
});

const ENVELOPE_STROKE = 0x5c7293;
const ENVELOPE_FILL = 0x0f1a2d;
const MIN_ENVELOPE_WIDTH = 144;
const MAX_ENVELOPE_WIDTH = 280;
const ENVELOPE_X_PADDING = 12;
const ENVELOPE_Y_PADDING = 10;
const ROW_HEIGHT = 34;
const VALUE_CHAR_LIMIT = 18;
const ROW_BG_FILL = 0x17263d;

const clampText = (text: string, maxChars: number): string => {
  if (text.length <= maxChars) {
    return text;
  }
  return `${text.slice(0, Math.max(0, maxChars - 3))}...`;
};

const signatureForRows = (
  rows: readonly { path: string; keyText: string; valueText: string; displayType: string }[],
): string => {
  return rows.map((row) => `${row.path}:${row.keyText}:${row.valueText}:${row.displayType}`).join("|");
};

export class PacketLayer extends Container {
  private readonly sprites = new Map<number, PacketSprite>();
  private frameCounter = 0;
  private laneCenterByPath: ReadonlyMap<string, number> = new Map();
  private focusPath: string | null = null;
  private focusDimAlpha = 0.4;

  public setLaneCenters(laneCenterByPath: ReadonlyMap<string, number>): void {
    this.laneCenterByPath = laneCenterByPath;
    for (const sprite of this.sprites.values()) {
      this.layoutRows(sprite);
    }
  }

  public setDimFocus(path: string | null, dimAlpha: number): void {
    this.focusPath = path;
    this.focusDimAlpha = dimAlpha;
    for (const sprite of this.sprites.values()) {
      this.layoutRows(sprite);
    }
  }

  public syncPackets(viewModels: readonly PacketViewModel[]): void {
    this.frameCounter += 1;

    for (const vm of viewModels) {
      let sprite = this.sprites.get(vm.id);
      if (sprite === undefined) {
        sprite = this.createPacketSprite(vm);
        this.sprites.set(vm.id, sprite);
        this.addChild(sprite.root);
      }

      sprite.lastTouchedFrame = this.frameCounter;
      sprite.root.position.set(vm.x, 0);
      sprite.root.alpha = vm.alpha;

      const signature = signatureForRows(vm.rows);
      const versionLabel = vm.versionLabel ?? "";
      if (signature !== sprite.rowSignature || versionLabel !== sprite.versionLabel) {
        sprite.versionLabel = versionLabel;
        this.rebuildRows(sprite, vm.rows);
      }
      this.layoutRows(sprite);
    }

    for (const [id, sprite] of this.sprites.entries()) {
      if (sprite.lastTouchedFrame === this.frameCounter) {
        continue;
      }
      this.removeChild(sprite.root);
      this.sprites.delete(id);
    }
  }

  public rowGlobalAnchor(packetId: number, path: string): Point | null {
    const sprite = this.sprites.get(packetId);
    if (sprite === undefined) {
      return null;
    }

    const row = sprite.rowsByPath.get(path);
    if (row === undefined) {
      return null;
    }

    const global = sprite.root.toGlobal(new Point(row.anchorX, row.centerY));
    return new Point(global.x, global.y);
  }

  public rowDisplayText(packetId: number, path: string): string | null {
    const sprite = this.sprites.get(packetId);
    if (sprite === undefined) {
      return null;
    }
    return sprite.rowsByPath.get(path)?.textValue ?? null;
  }

  private createPacketSprite(vm: PacketViewModel): PacketSprite {
    const root = new Container();
    const envelope = new Graphics();
    const versionTag = new Text(vm.versionLabel ?? "", versionTagStyle);
    root.addChild(envelope, versionTag);

    const sprite: PacketSprite = {
      root,
      envelope,
      versionTag,
      rowsByPath: new Map(),
      rowSignature: "",
      versionLabel: vm.versionLabel ?? "",
      lastTouchedFrame: this.frameCounter,
    };

    this.rebuildRows(sprite, vm.rows);
    return sprite;
  }

  private rebuildRows(
    sprite: PacketSprite,
    rows: readonly { path: string; keyText: string; valueText: string; displayType: string }[],
  ): void {
    sprite.root.removeChildren();
    sprite.root.addChild(sprite.envelope, sprite.versionTag);
    sprite.rowsByPath = new Map();
    sprite.rowSignature = signatureForRows(rows);
    sprite.versionTag.text = sprite.versionLabel;

    rows.forEach((row) => {
      const rowFontSize = 16;
      const keyStyle = new TextStyle({
        fill: 0xe8edf7,
        fontFamily: "Menlo, monospace",
        fontSize: rowFontSize,
        fontWeight: "600",
        dropShadow: true,
        dropShadowAlpha: 0.2,
        dropShadowBlur: 1,
        dropShadowDistance: 1,
        dropShadowColor: 0x000000,
      });
      const keyLabel = new Text(row.keyText, keyStyle);
      const clippedValue = clampText(row.valueText, VALUE_CHAR_LIMIT);
      const valueStyle = new TextStyle({
        fontFamily: "Menlo, monospace",
        fontSize: rowFontSize,
        fontWeight: "600",
        fill: colorForDisplayType(row.displayType),
        dropShadow: true,
        dropShadowAlpha: 0.2,
        dropShadowBlur: 1,
        dropShadowDistance: 1,
        dropShadowColor: 0x000000,
      });
      const valueLabel = new Text(clippedValue, valueStyle);
      const rowBg = new Graphics();
      const rowWidth = keyLabel.width + valueLabel.width;

      const chip: RowChipSprite = {
        path: row.path,
        textValue: `${row.keyText}${clippedValue}`,
        rowBg,
        keyLabel,
        valueLabel,
        rowWidth,
        rowHeight: ROW_HEIGHT,
        centerY: 0,
        anchorX: 0,
      };
      sprite.rowsByPath.set(row.path, chip);
      sprite.root.addChild(rowBg, keyLabel, valueLabel);
    });
  }

  private layoutRows(sprite: PacketSprite): void {
    const fallbackBase = this.fallbackBaseY();
    let fallbackIndex = 0;
    let minTop = Number.POSITIVE_INFINITY;
    let maxBottom = Number.NEGATIVE_INFINITY;
    let maxWidth = 0;
    let packetHasFocusPath = false;

    for (const row of sprite.rowsByPath.values()) {
      const laneY = this.laneCenterByPath.get(row.path);
      const centerY = laneY ?? (fallbackBase + fallbackIndex * 40);
      fallbackIndex += 1;
      row.centerY = centerY;

      const shouldDim = this.focusPath !== null && row.path !== this.focusPath;
      const alpha = shouldDim ? this.focusDimAlpha : 1;
      row.keyLabel.alpha = alpha;
      row.valueLabel.alpha = alpha;
      row.rowBg.alpha = alpha;
      if (this.focusPath !== null && row.path === this.focusPath) {
        packetHasFocusPath = true;
      }

      minTop = Math.min(minTop, centerY - row.rowHeight / 2);
      maxBottom = Math.max(maxBottom, centerY + row.rowHeight / 2);
      maxWidth = Math.max(maxWidth, row.rowWidth);
    }

    if (sprite.rowsByPath.size === 0) {
      sprite.envelope.clear();
      sprite.versionTag.visible = false;
      return;
    }

    const envelopeWidth = Math.max(
      MIN_ENVELOPE_WIDTH,
      Math.min(MAX_ENVELOPE_WIDTH, maxWidth + ENVELOPE_X_PADDING * 2),
    );
    const envelopeX = -envelopeWidth / 2;
    const envelopeY = minTop - ENVELOPE_Y_PADDING;
    const envelopeHeight = Math.max(44, maxBottom - minTop + ENVELOPE_Y_PADDING * 2);
    const rowStartX = envelopeX + 12;

    for (const row of sprite.rowsByPath.values()) {
      row.keyLabel.x = rowStartX;
      row.keyLabel.y = row.centerY - row.keyLabel.height / 2;
      row.valueLabel.x = row.keyLabel.x + row.keyLabel.width;
      row.valueLabel.y = row.centerY - row.valueLabel.height / 2;
      row.anchorX = Math.min(envelopeX + envelopeWidth - 6, rowStartX + row.rowWidth + 8);
      const bgWidth = Math.min(envelopeWidth - 16, row.rowWidth + 14);
      row.rowBg.clear();
      row.rowBg.beginFill(ROW_BG_FILL, 0.88);
      row.rowBg.drawRoundedRect(
        rowStartX - 7,
        row.centerY - row.rowHeight / 2 + 2,
        bgWidth,
        row.rowHeight - 4,
        6,
      );
      row.rowBg.endFill();
    }

    sprite.envelope.clear();
    sprite.envelope.lineStyle(1.9, ENVELOPE_STROKE, 0.8);
    sprite.envelope.beginFill(ENVELOPE_FILL, 0.72);
    sprite.envelope.drawRoundedRect(envelopeX, envelopeY, envelopeWidth, envelopeHeight, 10);
    sprite.envelope.endFill();

    const envelopeShouldDim = this.focusPath !== null && !packetHasFocusPath;
    const envelopeAlpha = envelopeShouldDim ? this.focusDimAlpha : 1;
    sprite.envelope.alpha = envelopeAlpha;

    sprite.versionTag.visible = sprite.versionLabel.length > 0;
    if (sprite.versionTag.visible) {
      sprite.versionTag.x = envelopeX + 8;
      sprite.versionTag.y = envelopeY - sprite.versionTag.height - 3;
      sprite.versionTag.alpha = envelopeAlpha;
    }
  }

  private fallbackBaseY(): number {
    const laneCenters = Array.from(this.laneCenterByPath.values()).sort((a, b) => a - b);
    if (laneCenters.length === 0) {
      return 220;
    }
    return laneCenters[0] ?? 220;
  }
}
