import type {
  DecodeEvent,
  Packet,
  RuntimeStory,
  SchemaVersion,
  StoryState,
} from "../model/types";
import { generatePayload } from "../schema/generate";
import { validatePayloadAgainstFields } from "../schema/validate";
import { getStateById } from "./transitions";

export interface EngineConfig {
  emitIntervalSec: number;
  packetSpeedPxPerSec: number;
  spawnX: number;
  decodeX: number;
  despawnX: number;
  packetY: number;
  initialPacketCount: number;
  initialPacketSpacing: number;
}

export class WireEngine {
  private readonly story: RuntimeStory;
  private config: EngineConfig;
  private packets: Packet[] = [];
  private decodeEvents: DecodeEvent[] = [];
  private nextPacketId = 1;
  private emitAccumulatorSec = 0;
  private currentStateId: string;
  private paused = false;

  public constructor(story: RuntimeStory, config: EngineConfig) {
    this.story = story;
    this.config = config;
    this.currentStateId = story.initialStateId;
    this.seedInitialPackets();
  }

  public isPaused(): boolean {
    return this.paused;
  }

  public togglePaused(): void {
    this.paused = !this.paused;
  }

  public reset(): void {
    this.currentStateId = this.story.initialStateId;
    this.emitAccumulatorSec = 0;
    this.nextPacketId = 1;
    this.decodeEvents = [];
    this.packets = [];
    this.seedInitialPackets();
  }

  public updateGeometry(
    geometry: Pick<EngineConfig, "spawnX" | "decodeX" | "despawnX" | "packetY">,
  ): void {
    this.config = {
      ...this.config,
      ...geometry,
    };
    for (const packet of this.packets) {
      packet.y = geometry.packetY;
    }
  }

  public stateId(): string {
    return this.currentStateId;
  }

  public state(): StoryState {
    return getStateById(this.story, this.currentStateId);
  }

  public activePackets(): readonly Packet[] {
    return this.packets;
  }

  public drainDecodeEvents(): readonly DecodeEvent[] {
    const out = this.decodeEvents;
    this.decodeEvents = [];
    return out;
  }

  public transitionTo(stateId: string): void {
    if (!this.story.states.has(stateId)) {
      throw new Error(`cannot transition to unknown state '${stateId}'`);
    }
    this.currentStateId = stateId;
  }

  public step(deltaSec: number): void {
    if (this.paused) {
      return;
    }

    for (const packet of this.packets) {
      packet.x += this.config.packetSpeedPxPerSec * deltaSec;
    }

    this.processDecodes();
    this.emitAccumulatorSec += deltaSec;

    while (this.emitAccumulatorSec >= this.config.emitIntervalSec) {
      this.emitAccumulatorSec -= this.config.emitIntervalSec;
      this.emitPacket(this.currentLeftVersion().id, this.config.spawnX);
    }

    this.packets = this.packets.filter((packet) => packet.x < this.config.despawnX);
  }

  private seedInitialPackets(): void {
    const leftVersionId = this.currentLeftVersion().id;
    for (let i = 0; i < this.config.initialPacketCount; i += 1) {
      const x = this.config.spawnX + i * this.config.initialPacketSpacing;
      this.emitPacket(leftVersionId, x);
    }
  }

  private emitPacket(schemaVersionId: string, x: number): void {
    const packetId = this.nextPacketId;
    this.nextPacketId += 1;
    this.packets.push({
      id: packetId,
      schemaVersionId,
      payload: generatePayload(this.version(schemaVersionId).schema, `${schemaVersionId}:${packetId}`),
      x,
      y: this.config.packetY,
    });
  }

  private processDecodes(): void {
    const rightVersion = this.currentRightVersion();
    const remainingPackets: Packet[] = [];
    for (const packet of this.packets) {
      if (packet.x < this.config.decodeX) {
        remainingPackets.push(packet);
        continue;
      }

      const validation = validatePayloadAgainstFields(packet.payload, rightVersion.fields);
      this.decodeEvents.push({
        packetId: packet.id,
        result: validation.result,
        matchedPaths: validation.matchedPaths,
      });
    }
    this.packets = remainingPackets;
  }

  private version(versionId: string): SchemaVersion {
    const version = this.story.versions.get(versionId);
    if (version === undefined) {
      throw new Error(`unknown version '${versionId}'`);
    }
    return version;
  }

  public currentLeftVersion(): SchemaVersion {
    return this.version(this.state().leftVersionId);
  }

  public currentRightVersion(): SchemaVersion {
    return this.version(this.state().rightVersionId);
  }
}
