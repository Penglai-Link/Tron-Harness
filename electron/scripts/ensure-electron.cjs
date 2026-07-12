#!/usr/bin/env node

const childProcess = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");

const { downloadArtifact } = require("@electron/get");

const electronDir = path.resolve(__dirname, "..", "node_modules", "electron");
const distDir = path.join(electronDir, "dist");
const pathFile = path.join(electronDir, "path.txt");
const packageJson = require(path.join(electronDir, "package.json"));

function platformPathFor(platform) {
  switch (platform) {
    case "mas":
    case "darwin":
      return "Electron.app/Contents/MacOS/Electron";
    case "freebsd":
    case "openbsd":
    case "linux":
      return "electron";
    case "win32":
      return "electron.exe";
    default:
      throw new Error(`Electron builds are not available on platform: ${platform}`);
  }
}

function resolveArch(platform) {
  let arch = process.env.npm_config_arch || process.arch;
  if (
    platform === "darwin"
    && process.platform === "darwin"
    && arch === "x64"
    && process.env.npm_config_arch === undefined
  ) {
    try {
      const output = childProcess.execSync("sysctl -in sysctl.proc_translated");
      if (output.toString().trim() === "1") {
        arch = "arm64";
      }
    } catch (_error) {
      // Not running under Rosetta, or sysctl is unavailable.
    }
  }
  return arch;
}

function isInstalled(platformPath) {
  try {
    const installedVersion = fs.readFileSync(path.join(distDir, "version"), "utf8").replace(/^v/, "");
    const installedPath = fs.readFileSync(pathFile, "utf8");
    return installedVersion === packageJson.version
      && installedPath === platformPath
      && fs.existsSync(path.join(distDir, platformPath));
  } catch (_error) {
    return false;
  }
}

function extractZip(zipPath) {
  fs.rmSync(distDir, { force: true, recursive: true });
  fs.mkdirSync(distDir, { recursive: true });

  if (process.platform === "win32") {
    childProcess.execFileSync(
      "powershell.exe",
      ["-NoProfile", "-Command", `Expand-Archive -LiteralPath '${zipPath}' -DestinationPath '${distDir}' -Force`],
      { stdio: "inherit" },
    );
    return;
  }

  childProcess.execFileSync("unzip", ["-oq", zipPath, "-d", distDir], { stdio: "inherit" });
}

async function main() {
  const platform = process.env.npm_config_platform || process.platform;
  const arch = resolveArch(platform);
  const platformPath = platformPathFor(platform);

  if (isInstalled(platformPath)) {
    console.log(`Electron ${packageJson.version} is ready for ${platform}-${arch}.`);
    return;
  }

  console.log(`Downloading Electron ${packageJson.version} for ${platform}-${arch}...`);
  const zipPath = await downloadArtifact({
    version: packageJson.version,
    artifactName: "electron",
    force: process.env.force_no_cache === "true",
    cacheRoot: process.env.electron_config_cache,
    checksums: (process.env.electron_use_remote_checksums || process.env.npm_config_electron_use_remote_checksums)
      ? undefined
      : require(path.join(electronDir, "checksums.json")),
    platform,
    arch,
  });

  console.log("Extracting Electron runtime...");
  extractZip(zipPath);

  const extractedTypeDefinition = path.join(distDir, "electron.d.ts");
  if (fs.existsSync(extractedTypeDefinition)) {
    fs.renameSync(extractedTypeDefinition, path.join(electronDir, "electron.d.ts"));
  }

  fs.writeFileSync(pathFile, platformPath);
  if (!isInstalled(platformPath)) {
    throw new Error("Electron runtime extraction did not produce a runnable binary");
  }
  console.log(`Electron ${packageJson.version} is ready for ${platform}-${arch}.`);
}

main().catch((error) => {
  console.error(error.stack || error.message);
  process.exit(1);
});
