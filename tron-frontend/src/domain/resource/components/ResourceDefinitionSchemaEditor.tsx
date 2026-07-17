import type { ResourceDefinitionAttributesSchema } from "@/domain/resource/model";
import { JsonSchemaDraft7Editor } from "@/shared";

interface ResourceDefinitionSchemaEditorProps {
  onChange: (schema: ResourceDefinitionAttributesSchema) => void;
  onValidityChange?: (valid: boolean) => void;
  readOnly: boolean;
  schema: ResourceDefinitionAttributesSchema;
}

export function ResourceDefinitionSchemaEditor({
  onChange,
  onValidityChange,
  readOnly,
  schema,
}: ResourceDefinitionSchemaEditorProps) {
  return (
    <JsonSchemaDraft7Editor
      formatOptions={[{ value: "tron-variable-ref" }]}
      jsonFileName="attributesSchema.json"
      readOnly={readOnly}
      schema={schema}
      onChange={(nextSchema) => onChange(nextSchema as ResourceDefinitionAttributesSchema)}
      onValidityChange={onValidityChange}
    />
  );
}
