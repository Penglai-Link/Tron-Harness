export interface MemoryFragment {
  id: string;
  content: string;
  namespace: string;
  entity: string;
  source: string;
  confidence: number;
  created: string;
  inheritance: "Local" | "Inherited" | "Override";
}
