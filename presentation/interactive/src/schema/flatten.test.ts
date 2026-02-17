import { describe, expect, it } from "vitest";
import { flattenJsonSchema } from "./flatten";
import type { JsonSchemaDocument } from "../model/types";

describe("flattenJsonSchema", () => {
  it("flattens nested objects and arrays with required metadata", () => {
    const schema: JsonSchemaDocument = {
      type: "object",
      properties: {
        user: {
          type: "object",
          properties: {
            name: { type: "string" },
            age: { type: "integer" },
          },
          required: ["name"],
        },
        city: { type: "string" },
        tags: {
          type: "array",
          items: { type: "string" },
        },
      },
      required: ["user", "city"],
    };

    const fields = flattenJsonSchema(schema);
    const byPath = new Map(fields.map((field) => [field.path, field]));

    expect(byPath.get("user.name")?.required).toBe(true);
    expect(byPath.get("user.age")?.required).toBe(false);
    expect(byPath.get("city")?.displayType).toBe("str");
    expect(byPath.get("tags")?.displayType).toBe("list[str]");
  });

  it("rejects unsupported unions with multiple non-null members", () => {
    const schema: JsonSchemaDocument = {
      type: "object",
      properties: {
        city: {
          type: ["string", "integer"],
        },
      },
      required: ["city"],
    };

    expect(() => flattenJsonSchema(schema)).toThrow(/unsupported type union/);
  });

  it("preserves conditional required fields under optional and nullable parent objects", () => {
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

    const fields = flattenJsonSchema(schema);
    const cityField = fields.find((field) => field.path === "profile.city");

    expect(cityField).toBeDefined();
    expect(cityField?.required).toBe(true);
    expect(cityField?.requiredWhenObjectPath).toBe("profile");
  });
});
