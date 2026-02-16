import { describe, expect, it } from "vitest";
import { FUN_CITIES, FUN_EYE_COLORS, FUN_INTERESTS, FUN_NAMES, generatePayload } from "./generate";
import type { JsonSchemaDocument } from "../model/types";

const schema: JsonSchemaDocument = {
  type: "object",
  properties: {
    name: { type: "string" },
    age: { type: "integer" },
    city: { type: "string" },
    interests: {
      type: "array",
      items: { type: "string" },
    },
  },
  required: ["name", "age", "city"],
};

const profileSchema: JsonSchemaDocument = {
  type: "object",
  properties: {
    name: { type: "string" },
    city: { type: "string" },
    eye_color: { type: "string" },
    interests: {
      type: "array",
      items: { type: "string" },
    },
  },
  required: ["name", "city", "eye_color", "interests"],
};

describe("generatePayload", () => {
  it("is deterministic by schema and seed", () => {
    const a = generatePayload(schema, "seed-a");
    const b = generatePayload(schema, "seed-a");
    const c = generatePayload(schema, "seed-b");

    expect(a).toEqual(b);
    expect(a).not.toEqual(c);
  });

  it("always includes required fields", () => {
    const payload = generatePayload(schema, "required-check");
    expect(payload).toHaveProperty("name");
    expect(payload).toHaveProperty("age");
    expect(payload).toHaveProperty("city");
  });

  it("uses seeded value pools for common profile strings", () => {
    const payload = generatePayload(profileSchema, "pool-check");
    expect(FUN_NAMES).toContain(payload.name as string);
    expect(FUN_CITIES).toContain(payload.city as string);
    expect(FUN_EYE_COLORS).toContain(payload.eye_color as string);
    expect(Array.isArray(payload.interests)).toBe(true);

    const interests = payload.interests as unknown[];
    for (const interest of interests) {
      expect(FUN_INTERESTS).toContain(interest as string);
    }
  });
});
