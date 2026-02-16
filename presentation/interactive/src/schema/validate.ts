import type {
  DecodeResult,
  FlattenedField,
  JsonSchemaDocument,
  ValueDescriptor,
} from "../model/types";
import { flattenJsonSchema } from "./flatten";

interface ValidationOutcome {
  result: DecodeResult;
  matchedPaths: readonly string[];
}

const isPlainObject = (value: unknown): value is Record<string, unknown> => {
  return typeof value === "object" && value !== null && !Array.isArray(value);
};

const matchesScalar = (value: unknown, scalar: ValueDescriptor & { kind: "scalar" }): boolean => {
  if (value === null) {
    return scalar.nullable;
  }

  switch (scalar.scalar) {
    case "string":
      return typeof value === "string";
    case "integer":
      return Number.isInteger(value);
    case "number":
      return typeof value === "number";
    case "boolean":
      return typeof value === "boolean";
    case "null":
      return value === null;
    default:
      return false;
  }
};

const matchesDescriptor = (value: unknown, descriptor: ValueDescriptor): boolean => {
  if (descriptor.kind === "scalar") {
    return matchesScalar(value, descriptor);
  }

  if (value === null) {
    return descriptor.nullable;
  }

  if (descriptor.kind === "object") {
    return isPlainObject(value);
  }

  if (!Array.isArray(value)) {
    return false;
  }

  return value.every((item) => matchesDescriptor(item, descriptor.item));
};

const getValueAtPath = (payload: Record<string, unknown>, path: string): { found: boolean; value: unknown } => {
  const parts = path.split(".");
  let current: unknown = payload;

  for (const part of parts) {
    if (!isPlainObject(current)) {
      return { found: false, value: undefined };
    }
    if (!(part in current)) {
      return { found: false, value: undefined };
    }
    current = current[part];
  }

  return { found: true, value: current };
};

const validateField = (
  payload: Record<string, unknown>,
  field: FlattenedField,
): DecodeResult | null => {
  const lookup = getValueAtPath(payload, field.path);

  if (!lookup.found) {
    if (field.required) {
      return {
        ok: false,
        failingPath: field.path,
        reason: "missing_required",
      };
    }
    return null;
  }

  if (!matchesDescriptor(lookup.value, field.descriptor)) {
    return {
      ok: false,
      failingPath: field.path,
      reason: "type_mismatch",
    };
  }

  return null;
};

export const validatePayloadAgainstFields = (
  payload: Record<string, unknown>,
  targetFields: readonly FlattenedField[],
): ValidationOutcome => {
  const matchedPaths: string[] = [];

  for (const field of targetFields) {
    const fieldResult = validateField(payload, field);
    if (fieldResult !== null) {
      return {
        result: fieldResult,
        matchedPaths,
      };
    }

    const lookup = getValueAtPath(payload, field.path);
    if (lookup.found) {
      matchedPaths.push(field.path);
    }
  }

  return {
    result: { ok: true },
    matchedPaths,
  };
};

export const validatePayload = (
  payload: Record<string, unknown>,
  targetSchema: JsonSchemaDocument,
): ValidationOutcome => {
  const targetFields = flattenJsonSchema(targetSchema);
  return validatePayloadAgainstFields(payload, targetFields);
};
