export interface GraphEntity {
  id: string;
  label: string;
  type: string;
  properties: Record<string, string>;
  relationships: Array<{ label: string; target: string }>;
  x: number;
  y: number;
}
