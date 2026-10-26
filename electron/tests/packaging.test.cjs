const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { execFileSync } = require("node:child_process");
const { validateConfiguration } = require("app-builder-lib/out/util/config/config.js");
const { build } = require("../package.json");

test("desktop packaging configuration passes electron-builder schema validation", async () => {
  await validateConfiguration(build, { isEnabled: false });
});

test("Linux packages include deb, rpm and AppImage with a tray icon", () => {
  assert.ok(build.linux.target.includes("deb"));
  assert.ok(build.linux.target.includes("rpm"));
  assert.ok(build.linux.target.includes("AppImage"));
  assert.ok(fs.statSync(path.resolve(__dirname, "..", build.linux.icon)).isFile());
  assert.ok(build.linux.maintainer);
  assert.ok(build.deb.recommends.includes("libayatana-appindicator3-1"));
});

test("Windows installer keeps the directory chooser and ships the runtime tray icon", () => {
  assert.equal(build.nsis.oneClick, false);
  assert.equal(build.nsis.allowToChangeInstallationDirectory, true);
  assert.ok(build.files.includes("assets/**/*"));
  const image = fs.readFileSync(path.resolve(__dirname, "..", build.win.icon));
  assert.equal(image.subarray(1, 4).toString(), "PNG");
  assert.equal(image.readUInt32BE(16), image.readUInt32BE(20), "Windows icon must be square");
  assert.ok(image.readUInt32BE(16) >= 256, "electron-builder requires a Windows icon of at least 256px");
  assert.ok(fs.statSync(path.resolve(__dirname, "..", build.nsis.include)).isFile());
});

test("NSIS removal preserves install-local data through reinstall and uninstall", { timeout: 30000 }, (t) => {
  if (process.platform !== "win32") return t.skip("requires Windows NSIS");
  const cacheDir = path.join(process.env.LOCALAPPDATA, "electron-builder", "Cache");
  if (!fs.existsSync(cacheDir)) return t.skip("electron-builder cache is unavailable");
  let compiler;
  for (const directory of fs.readdirSync(cacheDir).filter((name) => name.startsWith("nsis-"))) {
    const root = path.join(cacheDir, directory);
    for (const entry of fs.readdirSync(root, { withFileTypes: true }).filter((item) => item.isDirectory())) {
      const candidate = path.join(root, entry.name, "makensis.exe");
      if (fs.existsSync(candidate)) compiler = candidate;
    }
  }
  if (!compiler) return t.skip("NSIS compiler is not cached; run make desktop-build first");
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "tron-nsis-data-"));
  const installDir = path.join(root, "custom install", "Tron");
  const installer = path.join(root, "fixture.exe");
  const uninstaller = path.join(installDir, "Uninstall.exe");
  const dataFile = path.join(installDir, "data", "resources", "attachment.txt");
  try {
    fs.writeFileSync(path.join(root, "payload.txt"), "application fixture");
    fs.writeFileSync(path.join(root, "fixture.nsi"), `Unicode true
RequestExecutionLevel user
SilentInstall silent
SilentUnInstall silent
OutFile "${installer}"
InstallDir "${installDir}"
!include "${path.resolve(__dirname, "../build/installer.nsh")}"
Section
  SetOutPath "$INSTDIR"
  File /oname=Tron.exe "${path.join(root, "payload.txt")}"
  SetOutPath "$INSTDIR\\resources"
  File "${path.join(root, "payload.txt")}"
  WriteUninstaller "${uninstaller}"
SectionEnd
Section Uninstall
  !insertmacro customRemoveFiles
SectionEnd
`);
    execFileSync(compiler, ["/V2", path.join(root, "fixture.nsi")], { windowsHide: true, timeout: 10000 });
    for (let cycle = 0; cycle < 2; cycle++) {
      execFileSync(installer, ["/S"], { windowsHide: true, timeout: 10000 });
      if (cycle === 0) {
        fs.mkdirSync(path.dirname(dataFile), { recursive: true });
        fs.writeFileSync(dataFile, "user attachment");
      }
      assert.equal(fs.readFileSync(dataFile, "utf8"), "user attachment");
      assert.ok(fs.existsSync(path.join(installDir, "Tron.exe")));
      // NSIS consumes the entire unquoted remainder after _?=, including spaces.
      execFileSync(uninstaller, ["/S", `_?=${installDir}`], { windowsHide: true, windowsVerbatimArguments: true, timeout: 10000 });
      assert.equal(fs.readFileSync(dataFile, "utf8"), "user attachment");
      assert.ok(!fs.existsSync(path.join(installDir, "Tron.exe")));
      assert.ok(!fs.existsSync(path.join(installDir, "resources")));
    }
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});
