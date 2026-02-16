import type {
  JsonSchemaDocument,
  JsonSchemaNode,
  JsonType,
  RuntimeStory,
  SchemaVersion,
  SchemaVersionDefinition,
  SeedWireFrom,
  StoryDefinition,
  StoryState,
  StoryStateDefinition,
  StoryTransition,
  StoryTransitionDefinition,
} from "./types";
import { flattenJsonSchema } from "../schema/flatten";

const JSON_TYPES: ReadonlySet<JsonType> = new Set(["string", "integer", "number", "boolean", "null", "object", "array"]);

const assertObject = (value: unknown, context: string): Record<string, unknown> => {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error(`${context} must be an object`);
  }
  return value as Record<string, unknown>;
};

const assertString = (value: unknown, context: string): string => {
  if (typeof value !== "string" || value.length === 0) {
    throw new Error(`${context} must be a non-empty string`);
  }
  return value;
};

const isSeedWireFrom = (value: unknown): value is SeedWireFrom => {
  return value === "left_before" || value === "left_after" || value === "right_before" || value === "right_after";
};

const parseJsonType = (value: unknown, context: string): JsonType => {
  const typeName = assertString(value, context);
  if (!JSON_TYPES.has(typeName as JsonType)) {
    throw new Error(`${context} has unsupported JSON schema type '${typeName}'`);
  }
  return typeName as JsonType;
};

const isJsonPrimitive = (value: unknown): boolean => {
  return value === null || typeof value === "string" || typeof value === "number" || typeof value === "boolean";
};

const parseSchemaNode = (value: unknown, context: string): JsonSchemaNode => {
  const obj = assertObject(value, context);
  const parsed: JsonSchemaNode = {};

  if (obj.type !== undefined) {
    if (typeof obj.type === "string") {
      parsed.type = parseJsonType(obj.type, `${context}.type`);
    } else if (Array.isArray(obj.type)) {
      parsed.type = obj.type.map((item, idx) => parseJsonType(item, `${context}.type[${idx}]`));
    } else {
      throw new Error(`${context}.type must be string or string[]`);
    }
  }

  if (obj.properties !== undefined) {
    const properties = assertObject(obj.properties, `${context}.properties`);
    parsed.properties = {};
    for (const [key, child] of Object.entries(properties)) {
      parsed.properties[key] = parseSchemaNode(child, `${context}.properties.${key}`);
    }
  }

  if (obj.required !== undefined) {
    if (!Array.isArray(obj.required)) {
      throw new Error(`${context}.required must be string[]`);
    }
    parsed.required = obj.required.map((item, idx) => assertString(item, `${context}.required[${idx}]`));
  }

  if (obj.items !== undefined) {
    parsed.items = parseSchemaNode(obj.items, `${context}.items`);
  }

  if (obj.enum !== undefined) {
    if (!Array.isArray(obj.enum)) {
      throw new Error(`${context}.enum must be an array`);
    }
    const invalidIndex = obj.enum.findIndex((item) => !isJsonPrimitive(item));
    if (invalidIndex >= 0) {
      throw new Error(`${context}.enum[${invalidIndex}] must be a JSON primitive`);
    }
    parsed.enum = [...obj.enum];
  }

  if (obj.format !== undefined) {
    parsed.format = assertString(obj.format, `${context}.format`);
  }

  return parsed;
};

const parseSchemaDocument = (value: unknown, context: string): JsonSchemaDocument => {
  const parsed = parseSchemaNode(value, context);
  if (parsed.type !== "object") {
    throw new Error(`${context} root schema must have type=object`);
  }
  if (parsed.properties === undefined) {
    throw new Error(`${context} root schema must include properties`);
  }
  return parsed as JsonSchemaDocument;
};

const parseVersion = (value: unknown, index: number): SchemaVersionDefinition => {
  const obj = assertObject(value, `versions[${index}]`);
  return {
    id: assertString(obj.id, `versions[${index}].id`),
    schema: parseSchemaDocument(obj.schema, `versions[${index}].schema`),
  };
};

const parseState = (value: unknown, index: number): StoryStateDefinition => {
  const obj = assertObject(value, `states[${index}]`);
  return {
    id: assertString(obj.id, `states[${index}].id`),
    leftVersionId: assertString(obj.leftVersionId, `states[${index}].leftVersionId`),
    rightVersionId: assertString(obj.rightVersionId, `states[${index}].rightVersionId`),
  };
};

const parseTransition = (value: unknown, index: number): StoryTransitionDefinition => {
  const obj = assertObject(value, `transitions[${index}]`);
  const seedWireFrom = obj.seedWireFrom;
  if (!isSeedWireFrom(seedWireFrom)) {
    throw new Error(`transitions[${index}].seedWireFrom is invalid`);
  }

  return {
    id: assertString(obj.id, `transitions[${index}].id`),
    fromStateId: assertString(obj.fromStateId, `transitions[${index}].fromStateId`),
    toStateId: assertString(obj.toStateId, `transitions[${index}].toStateId`),
    seedWireFrom,
  };
};

export const parseStoryDefinition = (input: unknown): StoryDefinition => {
  const root = assertObject(input, "story");

  if (!Array.isArray(root.versions) || root.versions.length === 0) {
    throw new Error("story.versions must be a non-empty array");
  }
  if (!Array.isArray(root.states) || root.states.length === 0) {
    throw new Error("story.states must be a non-empty array");
  }
  if (!Array.isArray(root.transitions) || root.transitions.length === 0) {
    throw new Error("story.transitions must be a non-empty array");
  }

  const versions = root.versions.map((entry, idx) => parseVersion(entry, idx));
  const states = root.states.map((entry, idx) => parseState(entry, idx));
  const transitions = root.transitions.map((entry, idx) => parseTransition(entry, idx));
  const initialStateId = assertString(root.initialStateId, "story.initialStateId");

  return {
    versions,
    states,
    transitions,
    initialStateId,
  };
};

export const materializeStory = (definition: StoryDefinition): RuntimeStory => {
  const versions = new Map<string, SchemaVersion>();
  for (const versionDef of definition.versions) {
    if (versions.has(versionDef.id)) {
      throw new Error(`duplicate schema version id: '${versionDef.id}'`);
    }
    versions.set(versionDef.id, {
      id: versionDef.id,
      schema: versionDef.schema,
      fields: flattenJsonSchema(versionDef.schema),
    });
  }

  const states = new Map<string, StoryState>();
  for (const stateDef of definition.states) {
    if (!versions.has(stateDef.leftVersionId)) {
      throw new Error(`state '${stateDef.id}' references missing left version '${stateDef.leftVersionId}'`);
    }
    if (!versions.has(stateDef.rightVersionId)) {
      throw new Error(`state '${stateDef.id}' references missing right version '${stateDef.rightVersionId}'`);
    }
    if (states.has(stateDef.id)) {
      throw new Error(`duplicate state id: '${stateDef.id}'`);
    }
    states.set(stateDef.id, {
      id: stateDef.id,
      leftVersionId: stateDef.leftVersionId,
      rightVersionId: stateDef.rightVersionId,
    });
  }

  if (!states.has(definition.initialStateId)) {
    throw new Error(`initialStateId '${definition.initialStateId}' is not declared in states`);
  }

  const transitionsByFromState = new Map<string, StoryTransition>();
  const transitionIds = new Set<string>();
  for (const transitionDef of definition.transitions) {
    if (transitionIds.has(transitionDef.id)) {
      throw new Error(`duplicate transition id: '${transitionDef.id}'`);
    }
    transitionIds.add(transitionDef.id);

    if (!states.has(transitionDef.fromStateId)) {
      throw new Error(`transition '${transitionDef.id}' references missing fromState '${transitionDef.fromStateId}'`);
    }
    if (!states.has(transitionDef.toStateId)) {
      throw new Error(`transition '${transitionDef.id}' references missing toState '${transitionDef.toStateId}'`);
    }
    if (transitionsByFromState.has(transitionDef.fromStateId)) {
      throw new Error(`multiple transitions from state '${transitionDef.fromStateId}' are not supported`);
    }

    transitionsByFromState.set(transitionDef.fromStateId, {
      id: transitionDef.id,
      fromStateId: transitionDef.fromStateId,
      toStateId: transitionDef.toStateId,
      seedWireFrom: transitionDef.seedWireFrom,
    });
  }

  return {
    versions,
    states,
    orderedStates: definition.states.map((state) => state.id),
    transitionsByFromState,
    initialStateId: definition.initialStateId,
  };
};
