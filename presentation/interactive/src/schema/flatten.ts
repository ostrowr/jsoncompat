import type {
  FlattenedField,
  JsonScalarType,
  JsonSchemaDocument,
  JsonSchemaNode,
  JsonType,
  ValueDescriptor,
} from "../model/types";

const SCALAR_TYPES: ReadonlySet<JsonScalarType> = new Set([
  "string",
  "integer",
  "number",
  "boolean",
  "null",
]);

interface NormalizedType {
  primary: JsonType;
  nullable: boolean;
}

const inferTypeFromShape = (schema: JsonSchemaNode): JsonType | null => {
  if (schema.properties !== undefined) {
    return "object";
  }
  if (schema.items !== undefined) {
    return "array";
  }
  return null;
};

const normalizeType = (schema: JsonSchemaNode, path: string): NormalizedType => {
  const rawType = schema.type;
  if (typeof rawType === "string") {
    return { primary: rawType, nullable: false };
  }

  if (Array.isArray(rawType)) {
    if (rawType.length === 0) {
      throw new Error(`empty type union at '${path}'`);
    }
    const unique = new Set(rawType);
    const nullable = unique.delete("null");
    if (unique.size !== 1) {
      throw new Error(`unsupported type union at '${path}'`);
    }
    const primary = [...unique][0];
    if (primary === undefined) {
      throw new Error(`invalid type union at '${path}'`);
    }
    return { primary, nullable };
  }

  const inferred = inferTypeFromShape(schema);
  if (inferred !== null) {
    return { primary: inferred, nullable: false };
  }

  throw new Error(`missing type at '${path}'`);
};

const descriptorFromSchema = (schema: JsonSchemaNode, path: string): ValueDescriptor => {
  const normalized = normalizeType(schema, path);

  if (SCALAR_TYPES.has(normalized.primary as JsonScalarType)) {
    return {
      kind: "scalar",
      scalar: normalized.primary as JsonScalarType,
      nullable: normalized.nullable,
    };
  }

  if (normalized.primary === "array") {
    if (schema.items === undefined) {
      throw new Error(`array schema missing items at '${path}'`);
    }
    return {
      kind: "array",
      item: descriptorFromSchema(schema.items, `${path}[]`),
      nullable: normalized.nullable,
    };
  }

  return {
    kind: "object",
    nullable: normalized.nullable,
  };
};

const displayTypeForDescriptor = (descriptor: ValueDescriptor): string => {
  if (descriptor.kind === "object") {
    return "object";
  }

  if (descriptor.kind === "scalar") {
    const base =
      descriptor.scalar === "string"
        ? "str"
        : descriptor.scalar === "integer"
          ? "int"
          : descriptor.scalar === "boolean"
            ? "bool"
            : descriptor.scalar;
    return descriptor.nullable ? `${base} | null` : base;
  }

  const itemDisplay = displayTypeForDescriptor(descriptor.item);
  return descriptor.nullable ? `list[${itemDisplay}] | null` : `list[${itemDisplay}]`;
};

const pushField = (
  fields: FlattenedField[],
  path: string,
  required: boolean,
  schema: JsonSchemaNode,
): void => {
  const descriptor = descriptorFromSchema(schema, path);
  fields.push({
    path,
    required,
    descriptor,
    displayType: displayTypeForDescriptor(descriptor),
  });
};

const walkObject = (
  schema: JsonSchemaNode,
  prefix: string,
  ancestorRequired: boolean,
  fields: FlattenedField[],
): void => {
  const normalized = normalizeType(schema, prefix || "<root>");
  if (normalized.primary !== "object") {
    throw new Error(`expected object schema at '${prefix || "<root>"}'`);
  }

  const properties = schema.properties;
  if (properties === undefined) {
    throw new Error(`object schema missing properties at '${prefix || "<root>"}'`);
  }

  const required = new Set(schema.required ?? []);
  for (const [name, propertySchema] of Object.entries(properties)) {
    const path = prefix.length > 0 ? `${prefix}.${name}` : name;
    const fieldRequired = ancestorRequired && required.has(name);
    const propType = normalizeType(propertySchema, path);

    if (propType.primary === "object" && propertySchema.properties !== undefined) {
      walkObject(propertySchema, path, fieldRequired, fields);
      continue;
    }

    pushField(fields, path, fieldRequired, propertySchema);
  }
};

export const flattenJsonSchema = (schema: JsonSchemaDocument): readonly FlattenedField[] => {
  const fields: FlattenedField[] = [];
  walkObject(schema, "", true, fields);
  return fields;
};

export const flattenSchemaNode = (schema: JsonSchemaNode): readonly FlattenedField[] => {
  const normalized = normalizeType(schema, "<root>");
  if (normalized.primary !== "object") {
    throw new Error("root schema must be an object");
  }
  const root = schema as JsonSchemaDocument;
  return flattenJsonSchema(root);
};

export const resolveSchemaType = (schema: JsonSchemaNode, path: string): NormalizedType => {
  return normalizeType(schema, path);
};
