const fs = require("node:fs");
const path = require("node:path");

function resolveBundledCapabilityPacks(resourcesDir, { packaged = false, mode = "native", platform = process.platform, arch = process.arch } = {}) {
  if (!["native", "wsl2"].includes(mode) || (mode === "wsl2" && platform !== "win32")) throw new Error("invalid-capability-bundle-mode");
  const directory = path.join(resourcesDir, "capability-packs", mode);
  const catalogPath = path.join(directory, "catalog.json");
  if (!fs.existsSync(catalogPath) && !packaged) return undefined;
  const catalog = JSON.parse(fs.readFileSync(catalogPath, "utf8"));
  const expectedOs = mode === "wsl2" ? "linux" : ({ win32: "windows", darwin: "macos", linux: "linux" })[platform];
  const expectedArch = ({ x64: "x86_64", arm64: "aarch64" })[arch];
  if (!expectedOs || !expectedArch || catalog.schemaVersion !== 1 || catalog.targetOs !== expectedOs || catalog.targetArch !== expectedArch || catalog.mode !== mode) {
    throw new Error(`capability-bundle-platform-mismatch: ${platform}/${arch}/${mode}`);
  }
  return directory;
}

module.exports = { resolveBundledCapabilityPacks };
