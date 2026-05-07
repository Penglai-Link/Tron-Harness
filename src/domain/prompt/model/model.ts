export interface PromptVariable {
  name: string;
  type: string;
  sample: string;
}

export interface PromptRecord {
  id: string;
  name: string;
  category: string;
  inheritance: string;
  updated: string;
  owner: string;
  body: string;
  variables: Record<string, string>;
}
