import { Container, Graphics, Point, Text, TextStyle } from "pixi.js";

interface BaseEffect {
  root: Container;
  ttlSec: number;
  maxTtlSec: number;
}

interface TransferEffect extends BaseEffect {
  kind: "transfer";
  from: Point;
  to: Point;
}

interface FadeEffect extends BaseEffect {
  kind: "fade";
}

interface FailureTagEffect extends BaseEffect {
  kind: "failure_tag";
  start: Point;
}

type EffectInstance = TransferEffect | FadeEffect | FailureTagEffect;

const failStyle = new TextStyle({
  fill: 0xfb7185,
  fontFamily: "Menlo, monospace",
  fontSize: 24,
  fontWeight: "700",
});

const transferStyle = new TextStyle({
  fill: 0xe8edf7,
  fontFamily: "Menlo, monospace",
  fontSize: 12,
  fontWeight: "600",
});

const failureTagStyle = new TextStyle({
  fill: 0xfb7185,
  fontFamily: "Menlo, monospace",
  fontSize: 12,
  fontWeight: "700",
});

const lerp = (start: number, end: number, t: number): number => start + (end - start) * t;
const easeOutCubic = (t: number): number => 1 - Math.pow(1 - t, 3);

export class DecodeFxLayer extends Container {
  private readonly effects: EffectInstance[] = [];

  public addTransferChip(input: {
    from: Point;
    to: Point;
    label: string;
    color: number;
    ttlSec: number;
  }): void {
    const root = new Container();
    const shell = new Graphics();
    const text = new Text(input.label, transferStyle);
    const width = Math.max(92, text.width + 20);
    const height = 24;

    shell.lineStyle(2.2, input.color, 0.95);
    shell.beginFill(0x16263d, 0.98);
    shell.drawRoundedRect(-width / 2, -height / 2, width, height, 8);
    shell.endFill();

    text.x = -width / 2 + (width - text.width) / 2;
    text.y = -text.height / 2;

    root.position.set(input.from.x, input.from.y);
    root.addChild(shell, text);
    this.addChild(root);
    this.effects.push({
      kind: "transfer",
      root,
      ttlSec: input.ttlSec,
      maxTtlSec: input.ttlSec,
      from: new Point(input.from.x, input.from.y),
      to: new Point(input.to.x, input.to.y),
    });
  }

  public addFailMark(position: Point, ttlSec: number): void {
    const root = new Container();
    const text = new Text("X", failStyle);
    text.position.set(position.x - text.width / 2, position.y - text.height / 2);
    root.addChild(text);
    this.addChild(root);
    this.effects.push({ kind: "fade", root, ttlSec, maxTtlSec: ttlSec });
  }

  public addFailureReasonTag(input: { position: Point; label: string; ttlSec: number }): void {
    const root = new Container();
    const shell = new Graphics();
    const text = new Text(input.label, failureTagStyle);
    const width = Math.max(110, text.width + 16);
    const height = 24;

    shell.lineStyle(1.8, 0xfb7185, 0.95);
    shell.beginFill(0x1f1118, 0.95);
    shell.drawRoundedRect(-width / 2, -height / 2, width, height, 8);
    shell.endFill();

    text.x = -width / 2 + (width - text.width) / 2;
    text.y = -text.height / 2;

    root.position.set(input.position.x, input.position.y);
    root.addChild(shell, text);
    this.addChild(root);
    this.effects.push({
      kind: "failure_tag",
      root,
      ttlSec: input.ttlSec,
      maxTtlSec: input.ttlSec,
      start: new Point(input.position.x, input.position.y),
    });
  }

  public tick(deltaSec: number): void {
    for (let i = this.effects.length - 1; i >= 0; i -= 1) {
      const effect = this.effects[i];
      if (effect === undefined) {
        continue;
      }
      effect.ttlSec -= deltaSec;
      if (effect.ttlSec <= 0) {
        this.removeChild(effect.root);
        this.effects.splice(i, 1);
        continue;
      }

      const progress = 1 - effect.ttlSec / effect.maxTtlSec;
      if (effect.kind === "transfer") {
        const eased = easeOutCubic(progress);
        effect.root.position.set(
          lerp(effect.from.x, effect.to.x, eased),
          lerp(effect.from.y, effect.to.y, eased),
        );
        effect.root.alpha = Math.max(0.18, 1 - progress * 0.72);
      } else if (effect.kind === "failure_tag") {
        const eased = easeOutCubic(progress);
        effect.root.position.set(effect.start.x, effect.start.y - eased * 14);
        effect.root.alpha = Math.max(0.15, 1 - progress * 0.8);
      } else {
        effect.root.alpha = Math.max(0.15, effect.ttlSec / effect.maxTtlSec);
      }
    }
  }

  public clearAll(): void {
    for (const effect of this.effects) {
      this.removeChild(effect.root);
    }
    this.effects.length = 0;
  }
}
