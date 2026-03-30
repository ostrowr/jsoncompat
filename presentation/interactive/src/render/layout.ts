import type { LayoutMetrics } from "../model/types";

export const BACKGROUND_COLOR = 0x070d17;
export const LEFT_ACCENT = 0x22d3ee;
export const RIGHT_ACCENT = 0xa3e635;
export const OPTIONAL_ACCENT = 0xfacc15;
export const SUCCESS_ACCENT = 0x4ade80;
export const FAILURE_ACCENT = 0xfb7185;
export const WIRE_ACCENT = 0x475569;
export const PACKET_ACCENT = 0x38bdf8;

export const computeLayout = (
  width: number,
  height: number,
  scale = 1,
): LayoutMetrics => {
  const safeScale = Math.max(0.36, Math.min(1.2, scale));
  const panelWidth = Math.min(360, Math.max(190, width * 0.24 * safeScale));
  const panelHeight = Math.min(height - 128, Math.max(340, height * 0.74));
  const gutter = Math.max(10, 16 * safeScale);
  const leftPanelX = panelWidth / 2 + gutter;
  const rightPanelX = width - panelWidth / 2 - gutter;
  const panelY = height / 2;

  const wireStartX = leftPanelX + panelWidth / 2 + 12 * safeScale;
  const wireEndX = rightPanelX - panelWidth / 2 - 12 * safeScale;
  const wireWidth = Math.max(360, wireEndX - wireStartX);
  const wireX = (wireStartX + wireEndX) / 2;
  const wireHeight = Math.min(260, Math.max(140, panelHeight * 0.42));
  const wireY = panelY - 18 * safeScale;

  return {
    width,
    height,
    leftPanelX,
    rightPanelX,
    panelY,
    panelWidth,
    panelHeight,
    wireX,
    wireY,
    wireWidth,
    wireHeight,
    wireStartX,
    wireEndX,
    decodeX: wireEndX - 136,
    packetY: wireY,
  };
};
