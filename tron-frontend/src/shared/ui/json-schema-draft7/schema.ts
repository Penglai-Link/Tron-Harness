import type {
  JsonSchemaDraft7,
  JsonSchemaDraft7EditorType,
  JsonSchemaDraft7RootObject,
  JsonSchemaDraft7Type,
  JsonSchemaDraft7Value,
} from "./types";
import { JSON_SCHEMA_DRAFT_7_URI } from "./types";

const typeSpecificKeywords = new Set([
  "allOf",
  "anyOf",
  "const",
  "enum",
  "exclusiveMaximum",
  "exclusiveMinimum",
  "format",
  "items",
  "maxItems",
  "maxLength",
  "maxProperties",
  "maximum",
  "minItems",
  "minLength",
  "minProperties",
  "minimum",
  "multipleOf",
  "oneOf",
  "pattern",
  "patternProperties",
  "properties",
  "required",
  "additionalProperties",
  "uniqueItems",
]);

export function cloneJsonSchema<T>(schema: T): T {
  if (typeof structuredClone === "function") return structuredClone(schema);
  return JSON.parse(JSON.stringify(schema)) as T;
}

export function asSchemaObject(schema: JsonSchemaDraft7Value | undefined): JsonSchemaDraft7 {
  return schema && typeof schema === "object" && !Array.isArray(schema) ? schema : {};
}

export function schemaEditorType(schema: JsonSchemaDraft7Value): JsonSchemaDraft7EditorType {
  if (typeof schema === "boolean") return "object";
  if (Array.isArray(schema.allOf)) return "allOf";
  if (Array.isArray(schema.anyOf)) return "anyOf";
  if (Array.isArray(schema.oneOf)) return "oneOf";
  const type = Array.isArray(schema.type) ? schema.type[0] : schema.type;
  return type ?? "object";
}

export function createSchemaForType(type: JsonSchemaDraft7EditorType): JsonSchemaDraft7 {
  if (type === "allOf" || type === "anyOf" || type === "oneOf") {
    return { [type]: type === "allOf" ? [{ type: "object", properties: {} }] : [{ type: "string" }, { type: "number" }] };
  }
  if (type === "object") return { additionalProperties: false, properties: {}, required: [], type };
  if (type === "array") return { items: { type: "string" }, type };
  return { type };
}

export function replaceSchemaType(schema: JsonSchemaDraft7Value, type: JsonSchemaDraft7EditorType): JsonSchemaDraft7 {
  const current = asSchemaObject(schema);
  const preserved = Object.fromEntries(Object.entries(current).filter(([keyword]) => (
    keyword !== "type" && !typeSpecificKeywords.has(keyword)
  )));
  return { ...preserved, ...createSchemaForType(type) };
}

export function parseRootObjectSchema(value: string): JsonSchemaDraft7RootObject {
  const parsed = JSON.parse(value) as unknown;
  if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
    throw new Error("Schema must be a JSON object.");
  }
  const schema = parsed as JsonSchemaDraft7;
  if (schema.type !== "object") throw new Error("The root Schema type must be object.");
  if (!schema.properties || typeof schema.properties !== "object" || Array.isArray(schema.properties)) {
    throw new Error("The root Schema must contain a properties object.");
  }
  if (schema.required !== undefined && !Array.isArray(schema.required)) {
    throw new Error("The root Schema required keyword must be an array.");
  }

  return {
    ...schema,
    $schema: JSON_SCHEMA_DRAFT_7_URI,
    additionalProperties: schema.additionalProperties ?? false,
    properties: schema.properties,
    required: schema.required ?? [],
    type: "object",
  };
}

export function serializeJsonSchema(schema: JsonSchemaDraft7RootObject) {
  return JSON.stringify(schema, null, 2);
}

export function nextPropertyName(properties: Record<string, JsonSchemaDraft7Value>, prefix = "field") {
  let index = Object.keys(properties).length + 1;
  while (properties[`${prefix}${index}`]) index += 1;
  return `${prefix}${index}`;
}

export function renameProperty(
  properties: Record<string, JsonSchemaDraft7Value>,
  previousName: string,
  nextName: string,
) {
  return Object.fromEntries(Object.entries(properties).map(([name, schema]) => (
    name === previousName ? [nextName, schema] : [name, schema]
  )));
}

export function reorderProperty(
  properties: Record<string, JsonSchemaDraft7Value>,
  name: string,
  direction: -1 | 1,
) {
  const entries = Object.entries(properties);
  const index = entries.findIndex(([entryName]) => entryName === name);
  const target = index + direction;
  if (index < 0 || target < 0 || target >= entries.length) return properties;
  [entries[index], entries[target]] = [entries[target], entries[index]];
  return Object.fromEntries(entries);
}

function valueMatchesType(value: unknown, type: JsonSchemaDraft7Type) {
  if (type === "array") return Array.isArray(value);
  if (type === "object") return Boolean(value) && typeof value === "object" && !Array.isArray(value);
  if (type === "integer") return Number.isInteger(value);
  if (type === "number") return typeof value === "number" && Number.isFinite(value);
  if (type === "null") return value === null;
  return typeof value === type;
}

function formatPath(path: string, segment: string | number) {
  return typeof segment === "number" ? `${path}[${segment}]` : path ? `${path}.${segment}` : segment;
}

export function validateJsonValue(schemaValue: JsonSchemaDraft7Value, value: unknown, path = "$"): string[] {
  if (schemaValue === true) return [];
  if (schemaValue === false) return [`${path}: value is not allowed.`];
  const schema = schemaValue;
  const errors: string[] = [];

  if (schema.allOf) {
    errors.push(...schema.allOf.flatMap((option) => validateJsonValue(option, value, path)));
  }
  if (schema.anyOf && !schema.anyOf.some((option) => validateJsonValue(option, value, path).length === 0)) {
    errors.push(`${path}: value does not match any allowed schema.`);
  }
  if (schema.oneOf) {
    const matches = schema.oneOf.filter((option) => validateJsonValue(option, value, path).length === 0).length;
    if (matches !== 1) errors.push(`${path}: value must match exactly one schema.`);
  }
  if (schema.if) {
    const conditionMatches = validateJsonValue(schema.if, value, path).length === 0;
    if (conditionMatches && schema.then) errors.push(...validateJsonValue(schema.then, value, path));
    if (!conditionMatches && schema.else) errors.push(...validateJsonValue(schema.else, value, path));
  }

  const declaredTypes = Array.isArray(schema.type) ? schema.type : schema.type ? [schema.type] : [];
  if (declaredTypes.length && !declaredTypes.some((type) => valueMatchesType(value, type))) {
    errors.push(`${path}: expected ${declaredTypes.join(" or ")}.`);
    return errors;
  }
  if (schema.const !== undefined && JSON.stringify(schema.const) !== JSON.stringify(value)) {
    errors.push(`${path}: value does not match the required constant.`);
  }
  if (schema.enum && !schema.enum.some((candidate) => Object.is(candidate, value))) {
    errors.push(`${path}: value is not in the allowed set.`);
  }

  if (typeof value === "string") {
    if (schema.minLength !== undefined && value.length < schema.minLength) errors.push(`${path}: minimum length is ${schema.minLength}.`);
    if (schema.maxLength !== undefined && value.length > schema.maxLength) errors.push(`${path}: maximum length is ${schema.maxLength}.`);
    if (schema.pattern) {
      try {
        if (!new RegExp(schema.pattern).test(value)) errors.push(`${path}: value does not match the required pattern.`);
      } catch {
        errors.push(`${path}: schema contains an invalid pattern.`);
      }
    }
  }
  if (typeof value === "number") {
    if (schema.minimum !== undefined && value < schema.minimum) errors.push(`${path}: minimum is ${schema.minimum}.`);
    if (schema.maximum !== undefined && value > schema.maximum) errors.push(`${path}: maximum is ${schema.maximum}.`);
    if (schema.exclusiveMinimum !== undefined && value <= schema.exclusiveMinimum) errors.push(`${path}: must be greater than ${schema.exclusiveMinimum}.`);
    if (schema.exclusiveMaximum !== undefined && value >= schema.exclusiveMaximum) errors.push(`${path}: must be less than ${schema.exclusiveMaximum}.`);
    if (schema.multipleOf !== undefined && value % schema.multipleOf !== 0) errors.push(`${path}: must be a multiple of ${schema.multipleOf}.`);
  }
  if (Array.isArray(value)) {
    if (schema.minItems !== undefined && value.length < schema.minItems) errors.push(`${path}: minimum item count is ${schema.minItems}.`);
    if (schema.maxItems !== undefined && value.length > schema.maxItems) errors.push(`${path}: maximum item count is ${schema.maxItems}.`);
    if (schema.uniqueItems && new Set(value.map((item) => JSON.stringify(item))).size !== value.length) errors.push(`${path}: items must be unique.`);
    if (schema.items && !Array.isArray(schema.items)) {
      value.forEach((item, index) => errors.push(...validateJsonValue(schema.items as JsonSchemaDraft7Value, item, formatPath(path, index))));
    }
    if (schema.contains && !value.some((item, index) => validateJsonValue(schema.contains!, item, formatPath(path, index)).length === 0)) {
      errors.push(`${path}: no item matches the required contained schema.`);
    }
  }
  if (value && typeof value === "object" && !Array.isArray(value)) {
    const object = value as Record<string, unknown>;
    const properties = schema.properties ?? {};
    (schema.required ?? []).forEach((name) => {
      if (!(name in object)) errors.push(`${formatPath(path, name)}: required property is missing.`);
    });
    Object.entries(object).forEach(([name, childValue]) => {
      const childSchema = properties[name];
      if (childSchema !== undefined) {
        errors.push(...validateJsonValue(childSchema, childValue, formatPath(path, name)));
      } else if (schema.additionalProperties === false) {
        errors.push(`${formatPath(path, name)}: additional property is not allowed.`);
      } else if (schema.additionalProperties && typeof schema.additionalProperties === "object") {
        errors.push(...validateJsonValue(schema.additionalProperties, childValue, formatPath(path, name)));
      }
    });
  }
  return errors;
}

export function createJsonSample(schemaValue: JsonSchemaDraft7Value): unknown {
  if (typeof schemaValue === "boolean") return schemaValue ? {} : null;
  const schema = schemaValue;
  if (schema.default !== undefined) return cloneJsonSchema(schema.default);
  if (schema.const !== undefined) return cloneJsonSchema(schema.const);
  if (schema.enum?.length) return cloneJsonSchema(schema.enum[0]);
  if (schema.allOf?.length) return createJsonSample(schema.allOf[0]);
  if (schema.anyOf?.length) return createJsonSample(schema.anyOf[0]);
  if (schema.oneOf?.length) return createJsonSample(schema.oneOf[0]);

  const type = Array.isArray(schema.type) ? schema.type[0] : schema.type;
  if (type === "object" || (!type && schema.properties)) {
    const properties = schema.properties ?? {};
    return Object.fromEntries((schema.required ?? []).flatMap((name) => (
      properties[name] === undefined ? [] : [[name, createJsonSample(properties[name])]]
    )));
  }
  if (type === "array") {
    if (!schema.items || Array.isArray(schema.items)) return [];
    return Array.from({ length: Math.max(1, schema.minItems ?? 0) }, () => createJsonSample(schema.items as JsonSchemaDraft7Value));
  }
  if (type === "string") return schema.minLength ? "x".repeat(Math.max(1, schema.minLength)) : "value";
  if (type === "integer" || type === "number") return schema.minimum ?? (schema.exclusiveMinimum !== undefined ? schema.exclusiveMinimum + 1 : 0);
  if (type === "boolean") return true;
  if (type === "null") return null;
  return {};
}
