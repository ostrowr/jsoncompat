import type { Packet, PacketRenderRow, SchemaVersion } from "../model/types";

const MAX_SCALAR_CHARS = 15;
const MAX_ARRAY_CHARS = 24;

const ellipsize = (text: string, limit: number): string => {
  if (text.length <= limit) {
    return text;
  }
  if (limit <= 3) {
    return text.slice(0, limit);
  }
  return `${text.slice(0, limit - 3)}...`;
};

const compactScalar = (value: unknown): string => {
  if (typeof value === "string") {
    return `"${ellipsize(value, Math.max(4, MAX_SCALAR_CHARS - 2))}"`;
  }
  if (value === null) {
    return "null";
  }
  if (typeof value === "number" || typeof value === "boolean") {
    return String(value);
  }
  const encoded = JSON.stringify(value);
  if (encoded === undefined) {
    return "null";
  }
  return ellipsize(encoded, MAX_SCALAR_CHARS);
};

const compactArray = (values: unknown[]): string => {
  const preview = values.slice(0, 2).map((value) => compactScalar(value));
  const remaining = values.length - preview.length;
  const suffix = remaining > 0 ? ` +${remaining}` : "";
  const composed = `[${preview.join(", ")}]${suffix}`;
  return ellipsize(composed, MAX_ARRAY_CHARS);
};

const compactJson = (value: unknown): string => {
  if (Array.isArray(value)) {
    return compactArray(value);
  }
  return compactScalar(value);
};

const leafPath = (path: string): string => {
  const parts = path.split(".");
  return parts[parts.length - 1] ?? path;
};

const getAtPath = (payload: Record<string, unknown>, path: string): unknown => {
  const parts = path.split(".");
  let cursor: unknown = payload;
  for (const part of parts) {
    if (typeof cursor !== "object" || cursor === null || Array.isArray(cursor)) {
      return undefined;
    }
    cursor = (cursor as Record<string, unknown>)[part];
  }
  return cursor;
};

export const packetRows = (packet: Packet, sourceVersion: SchemaVersion): readonly PacketRenderRow[] => {
  const rows: PacketRenderRow[] = [];
  for (const field of sourceVersion.fields) {
    const value = getAtPath(packet.payload, field.path);
    if (value === undefined) {
      continue;
    }
    rows.push({
      path: field.path,
      keyText: `${leafPath(field.path)}: `,
      valueText: compactJson(value),
      displayType: field.displayType,
    });
  }
  return rows;
};
