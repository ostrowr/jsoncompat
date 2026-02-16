import { describe, expect, it } from "vitest";
import { WireEngine, type EngineConfig } from "./engine";
import { materializeStory } from "../model/story-loader";
import type { StoryDefinition } from "../model/types";

const buildStory = (): StoryDefinition => ({
  versions: [
    {
      id: "v1",
      schema: {
        type: "object",
        properties: {
          city: { type: "string" },
        },
        required: ["city"],
      },
    },
    {
      id: "v2",
      schema: {
        type: "object",
        properties: {
          city: {
            type: "array",
            items: { type: "string" },
          },
        },
        required: ["city"],
      },
    },
  ],
  states: [
    { id: "s1", leftVersionId: "v1", rightVersionIds: ["v1"] },
    { id: "s2", leftVersionId: "v2", rightVersionIds: ["v2"] },
    { id: "s3", leftVersionId: "v2", rightVersionIds: ["v1", "v2"] },
  ],
  transitions: [
    { id: "t1", fromStateId: "s1", toStateId: "s2", seedWireFrom: "left_before" },
    { id: "t2", fromStateId: "s2", toStateId: "s3", seedWireFrom: "left_before" },
  ],
  initialStateId: "s1",
});

const config: EngineConfig = {
  emitIntervalSec: 0.5,
  packetSpeedPxPerSec: 10,
  spawnX: 0,
  decodeX: 10_000,
  despawnX: 20_000,
  packetY: 0,
  initialPacketCount: 2,
  initialPacketSpacing: 30,
  minPacketGapPx: 1,
};

describe("WireEngine transitions", () => {
  it("keeps in-flight packets unchanged and emits new packets from new source schema", () => {
    const story = materializeStory(buildStory());
    const engine = new WireEngine(story, config);

    const before = engine.activePackets().map((packet) => packet.schemaVersionId);
    expect(before.every((id) => id === "v1")).toBe(true);

    engine.transitionTo("s2");
    engine.step(0.1);

    const afterTransition = engine.activePackets().map((packet) => packet.schemaVersionId);
    expect(afterTransition.slice(0, 2)).toEqual(["v1", "v1"]);

    engine.step(0.5);
    const afterEmit = engine.activePackets().map((packet) => packet.schemaVersionId);
    expect(afterEmit.includes("v2")).toBe(true);
  });

  it("emits one decode event and removes packet after a short decode trail", () => {
    const story = materializeStory(buildStory());
    const decodingConfig: EngineConfig = {
      ...config,
      decodeX: 5,
      despawnX: 500,
      initialPacketCount: 1,
      initialPacketSpacing: 20,
      emitIntervalSec: 999,
      packetSpeedPxPerSec: 10,
    };

    const engine = new WireEngine(story, decodingConfig);
    expect(engine.activePackets().length).toBe(1);

    engine.step(0.5);
    expect(engine.drainDecodeEvents().length).toBe(1);
    expect(engine.activePackets().length).toBe(1);

    engine.step(6);
    expect(engine.activePackets().length).toBe(0);
  });

  it("uses reader union variants and records matched reader version", () => {
    const story = materializeStory(buildStory());
    const decodingConfig: EngineConfig = {
      ...config,
      decodeX: 5,
      despawnX: 500,
      initialPacketCount: 1,
      initialPacketSpacing: 20,
      emitIntervalSec: 999,
      packetSpeedPxPerSec: 10,
    };

    const engine = new WireEngine(story, decodingConfig);
    engine.transitionTo("s3");
    engine.step(0.5);

    const decode = engine.drainDecodeEvents();
    expect(decode.length).toBe(1);
    expect(decode[0]?.result.ok).toBe(true);
    expect(decode[0]?.matchedReaderVersionId).toBe("v1");
  });

  it("does not emit a new packet until the spawn gap invariant is satisfied", () => {
    const story = materializeStory(buildStory());
    const gapConfig: EngineConfig = {
      ...config,
      emitIntervalSec: 0.1,
      initialPacketCount: 1,
      initialPacketSpacing: 40,
      minPacketGapPx: 140,
      packetSpeedPxPerSec: 10,
    };

    const engine = new WireEngine(story, gapConfig);
    expect(engine.activePackets().length).toBe(1);

    engine.step(0.1);
    expect(engine.activePackets().length).toBe(1);

    engine.step(0.1);
    expect(engine.activePackets().length).toBe(1);

    engine.step(13.9);
    expect(engine.activePackets().length).toBe(2);
  });

  it("seeds the initial packet train starting at spawn so the wire can be populated densely", () => {
    const story = materializeStory(buildStory());
    const engine = new WireEngine(story, {
      ...config,
      spawnX: 100,
      initialPacketCount: 3,
      initialPacketSpacing: 40,
      minPacketGapPx: 40,
    });

    expect(engine.activePackets().map((packet) => packet.x)).toEqual([100, 140, 180]);
  });
});
