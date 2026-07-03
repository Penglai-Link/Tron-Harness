export interface ParsedGoalCommand {
  isGoalCommand: boolean;
  content: string;
}

const goalCommandPattern = /^\/goal(?:\s+)([\s\S]+)$/i;
const emptyGoalCommandPattern = /^\/goal\s*$/i;

export function parseGoalCommand(raw: string): ParsedGoalCommand {
  const trimmed = raw.trim();
  const match = goalCommandPattern.exec(trimmed);
  if (match) {
    return {
      isGoalCommand: true,
      content: (match[1] ?? "").trim(),
    };
  }

  if (emptyGoalCommandPattern.test(trimmed)) {
    return {
      isGoalCommand: true,
      content: "",
    };
  }

  return {
    isGoalCommand: false,
    content: trimmed,
  };
}
