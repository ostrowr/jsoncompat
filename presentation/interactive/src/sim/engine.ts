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

const DECODE_TRAIL_PX = 56;

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
  private decodedPacketIds = new Set<number>();
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
    this.nextPacketId = 1;
    this.emitAccumulatorSec = 0;
    this.decodeEvents = [];
    this.decodedPacketIds = new Set<number>();
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

  public emitIntervalSec(): number {
    return this.config.emitIntervalSec;
  }

  public setEmitIntervalSec(intervalSec: number): void {
    const clamped = Math.max(0.2, intervalSec);
    this.config = {
      ...this.config,
      emitIntervalSec: clamped,
    };
    this.emitAccumulatorSec = Math.min(this.emitAccumulatorSec, clamped);
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

    const remaining: Packet[] = [];
    for (const packet of this.packets) {
      if (packet.x < this.config.despawnX) {
        remaining.push(packet);
      } else {
        this.decodedPacketIds.delete(packet.id);
      }
    }
    this.packets = remaining;
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
    const rightVersions = this.currentRightVersions();
    if (rightVersions.length === 0) {
      throw new Error(`state '${this.currentStateId}' has no reader versions`);
    }
    const remainingPackets: Packet[] = [];
    for (const packet of this.packets) {
      if (packet.x < this.config.decodeX) {
        remainingPackets.push(packet);
        continue;
      }

      if (!this.decodedPacketIds.has(packet.id)) {
        const validation = this.validateAgainstReaderVersions(packet, rightVersions);
        this.decodeEvents.push({
          packetId: packet.id,
          result: validation.result,
          matchedPaths: validation.matchedPaths,
          matchedReaderVersionId: validation.readerVersionId,
        });
        this.decodedPacketIds.add(packet.id);
      }

      if (packet.x < this.config.decodeX + DECODE_TRAIL_PX) {
        remainingPackets.push(packet);
      } else {
        this.decodedPacketIds.delete(packet.id);
      }
    }
    this.packets = remainingPackets;
  }

  private validateAgainstReaderVersions(
    packet: Packet,
    rightVersions: readonly SchemaVersion[],
  ): { readerVersionId: string; result: DecodeEvent["result"]; matchedPaths: readonly string[] } {
    let bestFailure: { readerVersionId: string; result: DecodeEvent["result"]; matchedPaths: readonly string[] } | null = null;

    for (const rightVersion of rightVersions) {
      const validation = validatePayloadAgainstFields(packet.payload, rightVersion.fields);
      if (validation.result.ok) {
        return {
          readerVersionId: rightVersion.id,
          result: validation.result,
          matchedPaths: validation.matchedPaths,
        };
      }

      if (
        bestFailure === null
        || validation.matchedPaths.length > bestFailure.matchedPaths.length
      ) {
        bestFailure = {
          readerVersionId: rightVersion.id,
          result: validation.result,
          matchedPaths: validation.matchedPaths,
        };
      }
    }

    if (bestFailure !== null) {
      return bestFailure;
    }

    throw new Error(`state '${this.currentStateId}' has no reader versions`);
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

  public currentRightVersions(): readonly SchemaVersion[] {
    return this.state().rightVersionIds.map((rightVersionId) => this.version(rightVersionId));
  }
}
