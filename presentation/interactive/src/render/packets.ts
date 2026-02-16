import { Container, Graphics, Point, Text, TextStyle } from "pixi.js";
import type { PacketViewModel } from "../model/types";
import { colorForDisplayType } from "./type-colors";

interface RowChipSprite {
  path: string;
  textValue: string;
  shell: Graphics;
  keyLabel: Text;
  valueLabel: Text;
  width: number;
  height: number;
  centerY: number;
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

const rowStyle = new TextStyle({
  fill: 0xe8edf7,
  fontFamily: "Menlo, monospace",
  fontSize: 14,
  fontWeight: "500",
});

const versionTagStyle = new TextStyle({
  fill: 0xc8d4e8,
  fontFamily: "Menlo, monospace",
  fontSize: 11,
  fontWeight: "700",
});

const PACKET_BG = 0x152338;
const ROW_STROKE = 0x4b607f;
const ENVELOPE_STROKE = 0x4b607f;
const ENVELOPE_FILL = 0x0f1a2d;
const MAX_ROW_WIDTH = 180;

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
  private denseMode = false;

  public setLaneCenters(laneCenterByPath: ReadonlyMap<string, number>): void {
    this.laneCenterByPath = laneCenterByPath;
  }

  public setDimFocus(path: string | null, dimAlpha: number): void {
    this.focusPath = path;
    this.focusDimAlpha = dimAlpha;
  }

  public setDenseMode(enabled: boolean): void {
    if (this.denseMode === enabled) {
      return;
    }
    this.denseMode = enabled;
    for (const sprite of this.sprites.values()) {
      sprite.rowSignature = "";
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

    const global = sprite.root.toGlobal(new Point(row.width / 2 + 8, row.centerY));
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
      const rowFontSize = this.denseMode ? 13 : 14;
      const keyStyle = new TextStyle({
        fill: 0xe8edf7,
        fontFamily: "Menlo, monospace",
        fontSize: rowFontSize,
        fontWeight: "500",
      });
      const keyLabel = new Text(row.keyText, keyStyle);
      const maxValueChars = Math.max(5, Math.floor((MAX_ROW_WIDTH - 20 - keyLabel.width) / 7.6));
      const clippedValue = clampText(row.valueText, maxValueChars);
      const valueStyle = new TextStyle({
        fontFamily: "Menlo, monospace",
        fontSize: rowFontSize,
        fontWeight: "500",
        fill: colorForDisplayType(row.displayType),
      });
      const valueLabel = new Text(clippedValue, valueStyle);
      const width = Math.min(MAX_ROW_WIDTH, Math.max(132, keyLabel.width + valueLabel.width + 20));
      const height = 34;
      const shell = new Graphics();

      shell.lineStyle(2.2, ROW_STROKE, 0.95);
      shell.beginFill(PACKET_BG, 0.98);
      shell.drawRoundedRect(-width / 2, -height / 2, width, height, 8);
      shell.endFill();

      const chip: RowChipSprite = {
        path: row.path,
        textValue: `${row.keyText}${clippedValue}`,
        shell,
        keyLabel,
        valueLabel,
        width,
        height,
        centerY: 0,
      };
      sprite.rowsByPath.set(row.path, chip);
      sprite.root.addChild(shell, keyLabel, valueLabel);
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

      row.shell.position.set(0, centerY);
      row.keyLabel.x = -row.width / 2 + 10;
      row.keyLabel.y = centerY - row.keyLabel.height / 2;
      row.valueLabel.x = row.keyLabel.x + row.keyLabel.width;
      row.valueLabel.y = centerY - row.valueLabel.height / 2;

      const shouldDim = this.focusPath !== null && row.path !== this.focusPath;
      const alpha = shouldDim ? this.focusDimAlpha : 1;
      row.shell.alpha = alpha;
      row.keyLabel.alpha = alpha;
      row.valueLabel.alpha = alpha;
      if (this.focusPath !== null && row.path === this.focusPath) {
        packetHasFocusPath = true;
      }

      minTop = Math.min(minTop, centerY - row.height / 2);
      maxBottom = Math.max(maxBottom, centerY + row.height / 2);
      maxWidth = Math.max(maxWidth, row.width);
    }

    if (sprite.rowsByPath.size === 0) {
      sprite.envelope.clear();
      sprite.versionTag.visible = false;
      return;
    }

    const padX = 10;
    const padY = 10;
    const envelopeX = -maxWidth / 2 - padX;
    const envelopeY = minTop - padY;
    const envelopeWidth = maxWidth + padX * 2;
    const envelopeHeight = Math.max(42, maxBottom - minTop + padY * 2);

    sprite.envelope.clear();
    sprite.envelope.lineStyle(1.6, ENVELOPE_STROKE, 0.72);
    sprite.envelope.beginFill(ENVELOPE_FILL, 0.22);
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
