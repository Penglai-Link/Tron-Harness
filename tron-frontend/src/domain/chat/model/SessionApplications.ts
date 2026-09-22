export interface SessionApplication {
  appCode: string;
  appName: string;
  isPrimary: boolean;
  configuredDirectory: string;
  workingDirectory: string | null;
  repositoryRoot: string | null;
  branchName: string | null;
  headCommit: string | null;
  worktreePath: string | null;
  isWorktree: boolean;
  status: "Ready" | "NotGit" | "MissingDirectory" | "Unavailable";
}

export interface SessionApplications {
  projectCode: string | null;
  primaryAppCode: string | null;
  workingDirectory: string | null;
  applications: SessionApplication[];
}
