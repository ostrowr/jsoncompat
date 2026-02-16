import { describe, expect, it } from "vitest";
import { computeLayout } from "./layout";

describe("computeLayout", () => {
  it("keeps the decode position far enough from the reader panel for a max-width packet envelope", () => {
    const maxPacketEnvelopeWidthPx = 280;

    for (const width of [1024, 1280, 1440]) {
      const layout = computeLayout(width, 518, 0.5);
      const readerPanelLeftEdge = layout.rightPanelX - layout.panelWidth / 2;
      const packetRightEdgeAtDecode = layout.decodeX + maxPacketEnvelopeWidthPx / 2;

      expect(packetRightEdgeAtDecode).toBeLessThanOrEqual(readerPanelLeftEdge);
    }
  });
});
