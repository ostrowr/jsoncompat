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

  it("requires nested required fields when optional parent object is present", () => {
    const schema: JsonSchemaDocument = {
      type: "object",
      properties: {
        profile: {
          type: ["object", "null"],
          properties: {
            city: { type: "string" },
          },
          required: ["city"],
        },
      },
      required: [],
    };

    const absentParent = validatePayload({}, schema);
    expect(absentParent.result.ok).toBe(true);

    const nullableParent = validatePayload({ profile: null }, schema);
    expect(nullableParent.result.ok).toBe(true);

    const presentParentMissingChild = validatePayload({ profile: {} }, schema);
    expect(presentParentMissingChild.result.ok).toBe(false);
    expect(presentParentMissingChild.result.failingPath).toBe("profile.city");
    expect(presentParentMissingChild.result.reason).toBe("missing_required");
  });

  it("validates object-valued fields themselves, not only descendant leaves", () => {
    const schema: JsonSchemaDocument = {
      type: "object",
      properties: {
        profile: {
          type: "object",
          properties: {},
          required: [],
        },
      },
      required: ["profile"],
    };

    const missingParent = validatePayload({}, schema);
    expect(missingParent.result.ok).toBe(false);
    expect(missingParent.result.failingPath).toBe("profile");
    expect(missingParent.result.reason).toBe("missing_required");

    const wrongParentType = validatePayload(
      { profile: [] } as unknown as Record<string, unknown>,
      schema,
    );
    expect(wrongParentType.result.ok).toBe(false);
    expect(wrongParentType.result.failingPath).toBe("profile");
    expect(wrongParentType.result.reason).toBe("type_mismatch");
  });

  it("accepts explicit null scalar fields", () => {
    const schema: JsonSchemaDocument = {
      type: "object",
      properties: {
        deleted_at: { type: "null" },
      },
      required: ["deleted_at"],
    };

    const valid = validatePayload({ deleted_at: null }, schema);
    expect(valid.result.ok).toBe(true);

    const invalid = validatePayload({ deleted_at: "never" }, schema);
    expect(invalid.result.ok).toBe(false);
    expect(invalid.result.failingPath).toBe("deleted_at");
    expect(invalid.result.reason).toBe("type_mismatch");
  });
});
