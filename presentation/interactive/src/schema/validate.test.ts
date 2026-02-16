import { describe, expect, it } from "vitest";
import { validatePayload } from "./validate";
import type { JsonSchemaDocument } from "../model/types";

const schemaV2: JsonSchemaDocument = {
  type: "object",
  properties: {
    name: { type: "string" },
    age: { type: "integer" },
    city: { type: "string" },
    eye_color: { type: "string" },
  },
  required: ["name", "age", "city", "eye_color"],
};

describe("validatePayload", () => {
  it("fails when required field is missing", () => {
    const payload = {
      name: "david",
      age: 31,
      city: "seattle",
    };

    const outcome = validatePayload(payload, schemaV2);
    expect(outcome.result.ok).toBe(false);
    expect(outcome.result.failingPath).toBe("eye_color");
    expect(outcome.result.reason).toBe("missing_required");
  });

  it("fails on type mismatch", () => {
    const payload = {
      name: "david",
      age: "31",
      city: "seattle",
      eye_color: "hazel",
    };

    const outcome = validatePayload(payload as unknown as Record<string, unknown>, schemaV2);
    expect(outcome.result.ok).toBe(false);
    expect(outcome.result.failingPath).toBe("age");
    expect(outcome.result.reason).toBe("type_mismatch");
  });

  it("accepts list[str] when schema expects array of strings", () => {
    const schema: JsonSchemaDocument = {
      type: "object",
      properties: {
        city: {
          type: "array",
          items: { type: "string" },
        },
      },
      required: ["city"],
    };

    const payload = { city: ["seattle", "paris"] };
    const outcome = validatePayload(payload, schema);
    expect(outcome.result.ok).toBe(true);
  });
});
