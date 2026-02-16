import { describe, expect, it } from "vitest";
import { materializeStory, parseStoryDefinition } from "./story-loader";

describe("story-loader", () => {
  it("parses and materializes a minimal valid story", () => {
    const raw = {
      versions: [
        {
          id: "v1",
          schema: {
            type: "object",
            properties: {
              name: { type: "string" },
            },
            required: ["name"],
          },
        },
      ],
      states: [{ id: "s1", leftVersionId: "v1", rightVersionId: "v1" }],
      transitions: [
        {
          id: "t1",
          fromStateId: "s1",
          toStateId: "s1",
          seedWireFrom: "left_before",
        },
      ],
      initialStateId: "s1",
    };

    const parsed = parseStoryDefinition(raw);
    const story = materializeStory(parsed);

    expect(story.initialStateId).toBe("s1");
    expect(story.versions.get("v1")?.fields.length).toBe(1);
  });

  it("rejects duplicate transitions from the same state", () => {
    const raw = {
      versions: [
        {
          id: "v1",
          schema: {
            type: "object",
            properties: {
              name: { type: "string" },
            },
            required: ["name"],
          },
        },
      ],
      states: [
        { id: "s1", leftVersionId: "v1", rightVersionId: "v1" },
        { id: "s2", leftVersionId: "v1", rightVersionId: "v1" },
      ],
      transitions: [
        { id: "t1", fromStateId: "s1", toStateId: "s2", seedWireFrom: "left_before" },
        { id: "t2", fromStateId: "s1", toStateId: "s1", seedWireFrom: "left_before" },
      ],
      initialStateId: "s1",
    };

    const parsed = parseStoryDefinition(raw);
    expect(() => materializeStory(parsed)).toThrow(/multiple transitions from state/);
  });
});
