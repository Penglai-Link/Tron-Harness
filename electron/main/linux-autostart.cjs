const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { APP_ID, STARTUP_ARG } = require("./desktop-constants.cjs");

function quoteDesktopExecutable(executable) {
  if (!path.isAbsolute(executable) || /[\r\n\0=]/.test(executable)) {
    throw new Error("Linux startup requires a valid absolute executable path");
  }
  // Desktop Entry string unescaping happens before Exec argument unquoting.
  const escaped = executable.replace(/\\/g, "\\\\\\\\")
    .replace(/["$`]/g, character => `\\\\${character}`).replace(/%/g, "%%");
  return `"${escaped}"`;
}

function readEntry(filename) {
  let content;
  try { content = fs.readFileSync(filename, "utf8"); }
  catch (error) { if (error.code === "ENOENT") return null; throw error; }
  const entries = {};
  let inDesktopEntry = false;
  for (const rawLine of content.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (line.startsWith("[")) inDesktopEntry = line === "[Desktop Entry]";
    else if (inDesktopEntry && !line.startsWith("#")) {
      const separator = line.indexOf("=");
      if (separator > 0) entries[line.slice(0, separator).trim()] = line.slice(separator + 1).trim();
    }
  }
  return entries;
}

function createLinuxAutostart({ dataDir, env = process.env, homeDir = os.homedir(), execPath = process.execPath }) {
  // AppImage's process.execPath points inside a temporary /tmp/.mount_* folder.
  const executable = env.APPIMAGE || execPath;
  const exec = `${quoteDesktopExecutable(executable)} ${STARTUP_ARG}`;
  const tryExec = executable.replace(/\\/g, "\\\\").replace(/\t/g, "\\t");
  const configDir = env.XDG_CONFIG_HOME && path.isAbsolute(env.XDG_CONFIG_HOME)
    ? env.XDG_CONFIG_HOME : path.join(homeDir, ".config");
  const filename = path.join(configDir, "autostart", `${APP_ID}.desktop`);
  const marker = path.join(dataDir, "linux-startup-initialized");

  function writeEntry(entries) {
    fs.mkdirSync(path.dirname(filename), { recursive: true });
    const content = `[Desktop Entry]\n${Object.entries(entries).map(([key, value]) => `${key}=${value}`).join("\n")}\n`;
    fs.writeFileSync(filename, content, { mode: 0o600 });
  }

  function isEnabled() {
    const entry = readEntry(filename);
    const desktops = (env.XDG_CURRENT_DESKTOP || "").split(":").filter(Boolean);
    if (entry?.OnlyShowIn && !desktops.some(desktop => entry.OnlyShowIn.split(";").includes(desktop))) return false;
    if (entry?.NotShowIn && desktops.some(desktop => entry.NotShowIn.split(";").includes(desktop))) return false;
    return Boolean(entry?.Type === "Application" && entry.Exec
      && entry.Hidden !== "true" && entry["X-GNOME-Autostart-enabled"] !== "false");
  }

  function setEnabled(enabled) {
    writeEntry({
      Type: "Application", Version: "1.0", Name: "Tron",
      Comment: "Tron desktop", Exec: exec, TryExec: tryExec, Terminal: "false",
      Hidden: enabled ? "false" : "true",
      "X-GNOME-Autostart-enabled": enabled ? "true" : "false"
    });
  }

  return {
    filename, isEnabled, setEnabled,
    initialize() {
      if (!fs.existsSync(marker)) {
        // Respect an existing entry disabled in the desktop's startup settings.
        if (!fs.existsSync(filename)) setEnabled(true);
        fs.mkdirSync(dataDir, { recursive: true });
        fs.writeFileSync(marker, "1\n");
      }
      // Keep enabled entries pointing at the current install/AppImage location.
      // An externally removed or disabled entry remains disabled on later runs.
      if (isEnabled()) {
        const entry = readEntry(filename);
        if (entry.Exec !== exec || entry.TryExec !== tryExec) {
          writeEntry({ ...entry, Exec: exec, TryExec: tryExec });
        }
      }
    }
  };
}

module.exports = { createLinuxAutostart, quoteDesktopExecutable };
