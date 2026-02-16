export type JsonPrimitive = string | number | boolean | null;

export type JsonScalarType = "string" | "integer" | "number" | "boolean" | "null";
export type JsonType = JsonScalarType | "object" | "array";

export interface JsonSchemaNode {
  type?: JsonType | JsonType[];
  properties?: Record<string, JsonSchemaNode>;
  required?: string[];
  items?: JsonSchemaNode;
  enum?: JsonPrimitive[];
  format?: string;
}

export interface JsonSchemaDocument extends JsonSchemaNode {
  type: "object";
  properties: Record<string, JsonSchemaNode>;
}

export type ValueDescriptor =
  | {
      kind: "scalar";
      scalar: JsonScalarType;
      nullable: boolean;
    }
  | {
      kind: "array";
      item: ValueDescriptor;
      nullable: boolean;
    }
  | {
      kind: "object";
      nullable: boolean;
    };

export interface FlattenedField {
  path: string;
  required: boolean;
  requiredWhenObjectPath?: string;
  descriptor: ValueDescriptor;
  displayType: string;
}

export interface SchemaVersionDefinition {
  id: string;
  schema: JsonSchemaDocument;
}

export interface StoryStateDefinition {
  id: string;
  leftVersionId: string;
  rightVersionIds: readonly string[];
}

export type SeedWireFrom = "left_before" | "left_after" | "right_before" | "right_after";

export interface StoryTransitionDefinition {
  id: string;
  fromStateId: string;
  toStateId: string;
  seedWireFrom: SeedWireFrom;
}

export interface StoryDefinition {
  versions: SchemaVersionDefinition[];
  states: StoryStateDefinition[];
  transitions: StoryTransitionDefinition[];
  initialStateId: string;
}

export interface SchemaVersion {
  id: string;
  schema: JsonSchemaDocument;
  fields: readonly FlattenedField[];
}

export interface StoryState {
  id: string;
  leftVersionId: string;
  rightVersionIds: readonly string[];
}

export interface StoryTransition {
  id: string;
  fromStateId: string;
  toStateId: string;
  seedWireFrom: SeedWireFrom;
}

export interface RuntimeStory {
  versions: ReadonlyMap<string, SchemaVersion>;
  states: ReadonlyMap<string, StoryState>;
  orderedStates: readonly string[];
  transitionsByFromState: ReadonlyMap<string, StoryTransition>;
  initialStateId: string;
}

export interface DecodeResult {
  ok: boolean;
  failingPath?: string;
  reason?: "missing_required" | "type_mismatch";
}

export interface Packet {
  id: number;
  schemaVersionId: string;
  payload: Record<string, unknown>;
  x: number;
  y: number;
}

export interface DecodeEvent {
  packetId: number;
  result: DecodeResult;
  matchedPaths: readonly string[];
  matchedReaderVersionId: string;
}

export interface PacketRenderRow {
  path: string;
  keyText: string;
  valueText: string;
  displayType: string;
}

export interface PacketViewModel {
  id: number;
  x: number;
  y: number;
  rows: readonly PacketRenderRow[];
  color: number;
  alpha: number;
  versionLabel?: string;
}

export interface LayoutMetrics {
  width: number;
  height: number;
  leftPanelX: number;
  rightPanelX: number;
  panelY: number;
  panelWidth: number;
  panelHeight: number;
  wireX: number;
  wireY: number;
  wireWidth: number;
  wireHeight: number;
  wireStartX: number;
  wireEndX: number;
  decodeX: number;
  packetY: number;
}
