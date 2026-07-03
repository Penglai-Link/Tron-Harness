import type {
  GraphNodeView,
  GraphPropertyValue,
} from "@/domain/knowledge-graph/model/KnowledgeGraphModel";

export const defaultGraphNodeLabelProperty = "title";

export function isEmbeddingVectorProperty(propertyName: string) {
  const normalized = propertyName.toLowerCase();
  return normalized === "embedding" || normalized.endsWith("embeddingjson");
}

function graphPropertyValueText(value: GraphPropertyValue | undefined) {
  if (value === null || value === undefined) return "";
  return String(value).trim();
}

export function graphNodePropertyOptions(nodes: GraphNodeView[]) {
  const properties = new Set<string>([defaultGraphNodeLabelProperty]);
  for (const node of nodes) {
    for (const propertyName of Object.keys(node.properties)) {
      if (propertyName.trim().length === 0 || isEmbeddingVectorProperty(propertyName)) continue;
      properties.add(propertyName);
    }
  }

  return Array.from(properties).sort((left, right) => {
    if (left === defaultGraphNodeLabelProperty) return -1;
    if (right === defaultGraphNodeLabelProperty) return 1;
    return left.localeCompare(right);
  });
}

export function resolveGraphNodeLabel(node: GraphNodeView, preferredProperty: string) {
  const normalizedProperty = preferredProperty.trim() || defaultGraphNodeLabelProperty;
  const preferredValue = graphPropertyValueText(node.properties[normalizedProperty]);
  if (preferredValue) return `${normalizedProperty}：${preferredValue}`;

  const titleValue = graphPropertyValueText(node.properties[defaultGraphNodeLabelProperty]);
  if (titleValue) return `${defaultGraphNodeLabelProperty}：${titleValue}`;

  return `nodeCode：${node.nodeCode}`;
}
