export const JSON_SCHEMA_DRAFT_7_URI = "http://json-schema.org/draft-07/schema#" as const;

export type JsonSchemaDraft7Type =
  | "array"
  | "boolean"
  | "integer"
  | "null"
  | "number"
  | "object"
  | "string";

export type JsonSchemaDraft7EditorType = JsonSchemaDraft7Type | "allOf" | "anyOf" | "oneOf";

export type JsonSchemaDraft7Value = boolean | JsonSchemaDraft7;

export interface JsonSchemaDraft7 {
  [keyword: string]: unknown;
  $comment?: string;
  $id?: string;
  $ref?: string;
  $schema?: string;
  additionalProperties?: boolean | JsonSchemaDraft7Value;
  allOf?: JsonSchemaDraft7Value[];
  anyOf?: JsonSchemaDraft7Value[];
  const?: unknown;
  contains?: JsonSchemaDraft7Value;
  default?: unknown;
  definitions?: Record<string, JsonSchemaDraft7Value>;
  dependencies?: Record<string, string[] | JsonSchemaDraft7Value>;
  description?: string;
  enum?: unknown[];
  examples?: unknown[];
  else?: JsonSchemaDraft7Value;
  exclusiveMaximum?: number;
  exclusiveMinimum?: number;
  format?: string;
  if?: JsonSchemaDraft7Value;
  items?: JsonSchemaDraft7Value | JsonSchemaDraft7Value[];
  maxItems?: number;
  maxLength?: number;
  maxProperties?: number;
  maximum?: number;
  minItems?: number;
  minLength?: number;
  minProperties?: number;
  minimum?: number;
  multipleOf?: number;
  not?: JsonSchemaDraft7Value;
  oneOf?: JsonSchemaDraft7Value[];
  pattern?: string;
  patternProperties?: Record<string, JsonSchemaDraft7Value>;
  properties?: Record<string, JsonSchemaDraft7Value>;
  propertyNames?: JsonSchemaDraft7Value;
  readOnly?: boolean;
  required?: string[];
  title?: string;
  then?: JsonSchemaDraft7Value;
  type?: JsonSchemaDraft7Type | JsonSchemaDraft7Type[];
  uniqueItems?: boolean;
  writeOnly?: boolean;
}

export interface JsonSchemaDraft7RootObject extends JsonSchemaDraft7 {
  $schema: typeof JSON_SCHEMA_DRAFT_7_URI;
  additionalProperties: boolean | JsonSchemaDraft7Value;
  properties: Record<string, JsonSchemaDraft7Value>;
  required: string[];
  type: "object";
}

export interface JsonSchemaDraft7FormatOption {
  label?: string;
  value: string;
}

export interface JsonSchemaDraft7EditorProps {
  formatOptions?: JsonSchemaDraft7FormatOption[];
  jsonFileName?: string;
  onChange: (schema: JsonSchemaDraft7RootObject) => void;
  onValidityChange?: (valid: boolean) => void;
  readOnly?: boolean;
  sampleData?: string;
  schema: JsonSchemaDraft7RootObject;
}
