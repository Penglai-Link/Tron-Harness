export function formatePath(path: string) {
  return `${path.trim()} `;
}

// 常见的 shell 内建命令和系统命令。未列出的 CLI 由首个 token 规则兜底。
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

// 优先识别常见的英文提问和请求，避免将它们当作未知 CLI 执行。
const NATURAL_LANGUAGE_PREFIX =
  /^(?:can|could|create|describe|explain|how|list|please|show|tell|what|when|where|who|why|would)\b/i;
// shell 命令名允许的字符范围；这里只判断输入形态，不检查命令是否真实存在。
const SHELL_EXECUTABLE_TOKEN = /^[A-Za-z0-9_][A-Za-z0-9_.:@%+,-]*$/;

/**
 * 按输入形态判断内容应交给 shell 还是 AI 对话。
 *
 * 该判断是客户端路由规则，不会查询 PATH 或验证命令是否可执行。
 * 返回 true 表示交给 shell，返回 false 表示作为自然语言处理。
 */
export function isShellCommand(command: string): boolean {
  const input = command.trim();

  // 空输入无需发起 AI 请求，外层执行逻辑会直接忽略它。
  if (!input) {
    return true;
  }

  // 相对路径、用户目录路径和绝对路径都按可执行文件处理。
  if (/^(?:\.{0,2}\/|~\/|\/)/.test(input)) {
    return true;
  }

  // 支持以环境变量赋值开头的命令，例如 FOO=bar npm test。
  if (/^[A-Za-z_][A-Za-z0-9_]*=/.test(input)) {
    return true;
  }

  // shell 连接符、管道和重定向符明确表示命令表达式。
  if (/(?:&&|\|\||[|;<>])/.test(input)) {
    return true;
  }

  const firstToken = input.split(/\s+/)[0];

  if (SHELL_COMMANDS.has(firstToken)) {
    return true;
  }

  // 自然语言前缀必须先于通用 token 规则判断。
  if (NATURAL_LANGUAGE_PREFIX.test(input)) {
    return false;
  }

  // 未知但形态合法的命令名也交给 shell，由 shell 返回是否存在。
  return SHELL_EXECUTABLE_TOKEN.test(firstToken);
}
