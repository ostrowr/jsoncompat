import type { LayoutMetrics } from "../model/types";

export const BACKGROUND_COLOR = 0x070d17;
export const LEFT_ACCENT = 0x22d3ee;
export const RIGHT_ACCENT = 0xa3e635;
export const OPTIONAL_ACCENT = 0xfacc15;
export const SUCCESS_ACCENT = 0x4ade80;
export const FAILURE_ACCENT = 0xfb7185;
export const WIRE_ACCENT = 0x475569;
export const PACKET_ACCENT = 0x38bdf8;

export const computeLayout = (width: number, height: number): LayoutMetrics => {
  const panelWidth = Math.min(380, Math.max(300, width * 0.27));
  const panelHeight = Math.min(580, Math.max(460, height * 0.76));
  const leftPanelX = panelWidth / 2 + 36;
  const rightPanelX = width - panelWidth / 2 - 36;
  const panelY = height / 2;

  const wireStartX = leftPanelX + panelWidth / 2 + 26;
  const wireEndX = rightPanelX - panelWidth / 2 - 26;
  const wireWidth = Math.max(360, wireEndX - wireStartX);
  const wireX = (wireStartX + wireEndX) / 2;
  const wireHeight = Math.min(240, Math.max(180, panelHeight * 0.34));
  const wireY = panelY - 40;

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
    decodeX: wireEndX - 28,
    packetY: wireY,
  };
};
