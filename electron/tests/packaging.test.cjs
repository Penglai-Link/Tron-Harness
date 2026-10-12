const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const { validateConfiguration } = require("app-builder-lib/out/util/config/config.js");
const { ICON_PATH } = require("../main/windows-desktop.cjs");
const { build } = require("../package.json");

test("desktop packaging configuration passes electron-builder schema validation", async () => {
  await validateConfiguration(build, { isEnabled: false });
});

test("Linux packages include deb, rpm and AppImage with a tray icon", () => {
  assert.ok(build.linux.target.includes("deb"));
  assert.ok(build.linux.target.includes("rpm"));
  assert.ok(build.linux.target.includes("AppImage"));
  assert.equal(path.resolve(__dirname, "..", build.linux.icon), ICON_PATH);
  assert.ok(build.linux.maintainer);
  assert.ok(build.deb.recommends.includes("libayatana-appindicator3-1"));
});

test("Windows installer keeps the directory chooser and ships the runtime tray icon", () => {
  assert.equal(build.nsis.oneClick, false);
  assert.equal(build.nsis.allowToChangeInstallationDirectory, true);
  assert.ok(build.files.includes("assets/**/*"));
  assert.equal(path.resolve(__dirname, "..", build.win.icon), ICON_PATH);
  const image = fs.readFileSync(ICON_PATH);
  assert.equal(image.subarray(1, 4).toString(), "PNG");
  assert.equal(image.readUInt32BE(16), image.readUInt32BE(20), "Windows icon must be square");
  assert.ok(image.readUInt32BE(16) >= 256, "electron-builder requires a Windows icon of at least 256px");
  assert.ok(fs.statSync(path.resolve(__dirname, "..", build.nsis.include)).isFile());
});
