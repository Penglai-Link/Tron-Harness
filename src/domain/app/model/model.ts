export interface AppRecord {
  id: string;
  name: string;
  source: string;
  path: string;
  worktree: string;
  tags: string[];
  updated: string;
}

export interface ImportApplicationDefaults {
  source: "github" | "gitlab" | "local";
  repositoryUrl: string;
  accessToken: string;
  localPath: string;
  appName: string;
  tags: string[];
  branch: string;
}
