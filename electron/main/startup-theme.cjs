const fs = require("node:fs");
const path = require("node:path");
const modes = new Set(["light", "dark", "system"]);
const preferencePath = (app) => path.join(app.getPath("userData"), "startup-color-mode.json");
function readStartupColorMode(app) {
  try {
    const mode = JSON.parse(fs.readFileSync(preferencePath(app), "utf8")).colorMode;
    return modes.has(mode) ? mode : "system";
  } catch { return "system"; }
}
function saveStartupColorMode(app, colorMode) {
  if (!modes.has(colorMode)) return;
  try { fs.writeFileSync(preferencePath(app), JSON.stringify({ colorMode })); }
  catch (error) { console.warn("Could not persist startup display mode:", error.message); }
}
const startupTheme = (mode, nativeTheme) => mode === "dark" || (mode === "system" && nativeTheme.shouldUseDarkColors) ? "matrix-dark" : "light";
module.exports = { readStartupColorMode, saveStartupColorMode, startupTheme };
