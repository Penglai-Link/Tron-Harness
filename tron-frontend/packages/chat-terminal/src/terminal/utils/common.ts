export function formatePath(path: string) {
  return `${path.trim()} `;
}

const SHELL_COMMANDS = new Set([
  "awk",
  "cat",
  "cd",
  "chmod",
  "chown",
  "clear",
  "cp",
  "curl",
  "df",
  "dig",
  "du",
  "echo",
  "env",
  "export",
  "find",
  "grep",
  "head",
  "history",
  "ifconfig",
  "kill",
  "less",
  "ls",
  "mkdir",
  "mv",
  "netstat",
  "nslookup",
  "ping",
  "ping6",
  "ps",
  "pwd",
  "rm",
  "rmdir",
  "sed",
  "ssh",
  "tail",
  "tar",
  "top",
  "touch",
  "traceroute",
  "uname",
  "vim",
  "vi",
  "wget",
  "which",
]);

const NATURAL_LANGUAGE_PREFIX =
  /^(?:can|could|create|describe|explain|how|list|please|show|tell|what|when|where|who|why|would)\b/i;
const SHELL_EXECUTABLE_TOKEN = /^[A-Za-z0-9_][A-Za-z0-9_.:@%+,-]*$/;

// 判断是否是shell命令
export function isShellCommand(command: string): boolean {
  const input = command.trim();

  if (!input) {
    return true;
  }

  if (/^(?:\.{0,2}\/|~\/|\/)/.test(input)) {
    return true;
  }

  if (/^[A-Za-z_][A-Za-z0-9_]*=/.test(input)) {
    return true;
  }

  if (/(?:&&|\|\||[|;<>])/.test(input)) {
    return true;
  }

  const firstToken = input.split(/\s+/)[0];

  if (SHELL_COMMANDS.has(firstToken)) {
    return true;
  }

  if (NATURAL_LANGUAGE_PREFIX.test(input)) {
    return false;
  }

  return SHELL_EXECUTABLE_TOKEN.test(firstToken);
}
