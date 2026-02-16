import type { Packet, PacketRenderRow, SchemaVersion } from "../model/types";

const MAX_VALUE_CHARS = 16;

const trimValue = (value: unknown): string => {
  const encoded = JSON.stringify(value);
  if (encoded === undefined) {
    return "null";
  }
  if (encoded.length <= MAX_VALUE_CHARS) {
    return encoded;
  }
  return `${encoded.slice(0, MAX_VALUE_CHARS - 3)}...`;
};

const compactArray = (values: unknown[]): string => {
  const preview = values.slice(0, 2).map((value) => trimValue(value));
  const remaining = values.length - preview.length;
  const suffix = remaining > 0 ? `,+${remaining}` : "";
  return `[${preview.join(",")}${suffix}]`;
};

const compactJson = (value: unknown): string => {
  if (Array.isArray(value)) {
    return compactArray(value);
  }
  return trimValue(value);
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
