import type {
  FlattenedField,
  JsonPrimitive,
  JsonSchemaDocument,
  JsonScalarType,
  ValueDescriptor,
} from "../model/types";
import { flattenJsonSchema } from "./flatten";

export const FUN_NAMES = [
  "Nova",
  "River",
  "Ziggy",
  "Atlas",
  "Luna",
  "Milo",
  "Cleo",
  "Orion",
  "Piper",
  "Juno",
  "Niko",
  "Freya",
  "Kai",
  "Indie",
  "Remy",
  "Sable",
] as const;

export const FUN_CITIES = [
  "Tokyo",
  "Reykjavik",
  "Lisbon",
  "Seoul",
  "Auckland",
  "Vancouver",
  "Cape Town",
  "Kyoto",
  "Barcelona",
  "Melbourne",
  "Oslo",
  "Mexico City",
  "Montreal",
  "Portland",
  "Valencia",
  "Singapore",
] as const;

export const FUN_EYE_COLORS = [
  "hazel",
  "amber",
  "gray",
  "green",
  "brown",
  "blue",
  "violet",
  "gold",
] as const;

export const FUN_INTERESTS = [
  "climbing",
  "ceramics",
  "robotics",
  "street_food",
  "jazz",
  "stargazing",
  "surfing",
  "photography",
  "board_games",
  "trail_running",
  "origami",
  "gardening",
  "synth_music",
  "film",
  "birdwatching",
  "cooking",
  "comics",
  "coding",
  "backpacking",
  "fencing",
] as const;

const COUNTRY_POOL = ["usa", "japan", "canada", "spain", "france", "italy"] as const;

const hashText = (text: string): number => {
  let hash = 2166136261;
  for (let i = 0; i < text.length; i += 1) {
    hash ^= text.charCodeAt(i);
    hash = Math.imul(hash, 16777619);
  }
  return Math.abs(hash >>> 0);
};

const pickFrom = <T>(items: readonly [T, ...T[]], checksum: number): T => {
  const picked = items[checksum % items.length];
  if (picked === undefined) {
    throw new Error("non-empty value pool lookup failed");
  }
  return picked;
};

const leafKey = (path: string): string => {
  const leaf = path.split(".").at(-1) ?? path;
  return leaf.replace(/\[\d+\]/g, "");
};

const scalarValue = (scalar: JsonScalarType, path: string, seed: string): JsonPrimitive => {
  const token = `${path}:${seed}`;
  const checksum = hashText(token);
  const leaf = leafKey(path);

  switch (scalar) {
    case "string": {
      if (leaf === "name") {
        return pickFrom(FUN_NAMES, checksum);
      }
      if (leaf === "city") {
        return pickFrom(FUN_CITIES, checksum);
      }
      if (leaf === "eye_color") {
        return pickFrom(FUN_EYE_COLORS, checksum);
      }
      if (leaf === "interest" || leaf === "interests") {
        return pickFrom(FUN_INTERESTS, checksum);
      }
      if (leaf === "country") {
        return pickFrom(COUNTRY_POOL, checksum);
      }
      return `v${checksum % 100}`;
    }
    case "integer":
      return (checksum % 80) + 18;
    case "number":
      return Number(((checksum % 9000) / 100).toFixed(2));
    case "boolean":
      return checksum % 2 === 0;
    case "null":
      return null;
    default:
      return null;
  }
};

const valueForDescriptor = (descriptor: ValueDescriptor, path: string, seed: string): unknown => {
  if (descriptor.kind === "scalar") {
    return scalarValue(descriptor.scalar, path, seed);
  }

  if (descriptor.kind === "object") {
    return {};
  }

  const checksum = hashText(`${path}:${seed}:array`);
  const count = (checksum % 2) + 2;
  const values: unknown[] = [];
  for (let i = 0; i < count; i += 1) {
    values.push(valueForDescriptor(descriptor.item, `${path}[${i}]`, seed));
  }
  return values;
};

const setAtPath = (target: Record<string, unknown>, path: string, value: unknown): void => {
  const parts = path.split(".");
  let cursor: Record<string, unknown> = target;

  for (let i = 0; i < parts.length - 1; i += 1) {
    const part = parts[i];
    if (part === undefined) {
      continue;
    }
    const next = cursor[part];
    if (typeof next === "object" && next !== null && !Array.isArray(next)) {
      cursor = next as Record<string, unknown>;
      continue;
    }
    const created: Record<string, unknown> = {};
    cursor[part] = created;
    cursor = created;
  }

  const final = parts[parts.length - 1];
  if (final === undefined) {
    return;
  }
  cursor[final] = value;
};

const shouldIncludeOptional = (field: FlattenedField, seed: string): boolean => {
  if (field.required) {
    return true;
  }
  const checksum = hashText(`${seed}:${field.path}:optional`);
  return checksum % 100 < 65;
};

const isObjectPresentAtPath = (payload: Record<string, unknown>, path: string): boolean => {
  const parts = path.split(".");
  let cursor: unknown = payload;

  for (const part of parts) {
    if (typeof cursor !== "object" || cursor === null || Array.isArray(cursor)) {
      return false;
    }
    if (!(part in cursor)) {
      return false;
    }
    cursor = (cursor as Record<string, unknown>)[part];
  }

  return typeof cursor === "object" && cursor !== null && !Array.isArray(cursor);
};

export const generatePayloadFromFields = (
  fields: readonly FlattenedField[],
  seed: string,
): Record<string, unknown> => {
  const output: Record<string, unknown> = {};
  const deferredConditionalRequired: FlattenedField[] = [];

  for (const field of fields) {
    if (field.required && field.requiredWhenObjectPath !== undefined) {
      deferredConditionalRequired.push(field);
      continue;
    }
    if (!shouldIncludeOptional(field, seed)) {
      continue;
    }
    const value = valueForDescriptor(field.descriptor, field.path, seed);
    setAtPath(output, field.path, value);
  }

  for (const field of deferredConditionalRequired) {
    const requiredWhenObjectPath = field.requiredWhenObjectPath;
    if (requiredWhenObjectPath === undefined) {
      continue;
    }
    if (!isObjectPresentAtPath(output, requiredWhenObjectPath)) {
      continue;
    }
    const value = valueForDescriptor(field.descriptor, field.path, seed);
    setAtPath(output, field.path, value);
  }

  return output;
};

export const generatePayload = (
  schema: JsonSchemaDocument,
  seed: string,
): Record<string, unknown> => {
  const fields = flattenJsonSchema(schema);
  return generatePayloadFromFields(fields, seed);
};
