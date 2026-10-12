const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { createLinuxAutostart, quoteDesktopExecutable } = require("../main/linux-autostart.cjs");

function fixture(t, options = {}) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "tron-autostart-"));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  const settings = {
    dataDir: path.join(root, "data"), homeDir: path.join(root, "home"),
    execPath: "/opt/Tron/tron", env: {}, ...options
  };
  return { settings, startup: createLinuxAutostart(settings) };
}

test("Linux first run registers a per-user login entry and never re-enables a disabled entry", t => {
  const { settings, startup } = fixture(t);
  assert.equal(startup.filename, path.join(settings.homeDir, ".config/autostart/com.tron.desktop.desktop"));
  startup.initialize();
  assert.equal(startup.isEnabled(), true);
  assert.match(fs.readFileSync(startup.filename, "utf8"), /Exec="\/opt\/Tron\/tron" --tron-startup/);
  startup.setEnabled(false);
  createLinuxAutostart(settings).initialize();
  assert.equal(startup.isEnabled(), false);
  fs.rmSync(startup.filename);
  createLinuxAutostart(settings).initialize();
  assert.equal(fs.existsSync(startup.filename), false, "externally removed entries stay removed");
});

test("Linux honors XDG_CONFIG_HOME and uses the original AppImage path", t => {
  const { settings } = fixture(t);
  const startup = createLinuxAutostart({ ...settings,
    env: { XDG_CONFIG_HOME: path.join(settings.homeDir, "custom-config"), APPIMAGE: "/home/alice/应用/Tron Desktop.AppImage" },
    execPath: "/tmp/.mount_Tron123/tron"
  });
  startup.initialize();
  assert.equal(startup.filename, path.join(settings.homeDir, "custom-config/autostart/com.tron.desktop.desktop"));
  const entry = fs.readFileSync(startup.filename, "utf8");
  assert.match(entry, /Exec="\/home\/alice\/应用\/Tron Desktop.AppImage" --tron-startup/);
  assert.doesNotMatch(entry, /\.mount_/);
});

test("relative XDG_CONFIG_HOME is ignored", t => {
  const { settings, startup } = fixture(t, { env: { XDG_CONFIG_HOME: "relative" } });
  assert.equal(startup.filename, path.join(settings.homeDir, ".config/autostart/com.tron.desktop.desktop"));
});

test("moving an enabled AppImage updates the launch path without dropping desktop settings", t => {
  const { settings, startup } = fixture(t);
  startup.initialize();
  fs.appendFileSync(startup.filename, "X-GNOME-Autostart-Delay=10\n");
  const moved = createLinuxAutostart({ ...settings, env: { APPIMAGE: "/home/alice/Tron-new.AppImage" } });
  moved.initialize();
  const entry = fs.readFileSync(startup.filename, "utf8");
  assert.match(entry, /Exec="\/home\/alice\/Tron-new.AppImage" --tron-startup/);
  assert.match(entry, /TryExec=\/home\/alice\/Tron-new.AppImage/);
  assert.match(entry, /X-GNOME-Autostart-Delay=10/);
});

for (const disabledKey of ["Hidden=true", "X-GNOME-Autostart-enabled=false"]) {
  test(`first run respects an externally disabled startup entry: ${disabledKey}`, t => {
    const { startup } = fixture(t);
    fs.mkdirSync(path.dirname(startup.filename), { recursive: true });
    fs.writeFileSync(startup.filename, `[Desktop Entry]\nType=Application\nExec=/old/tron\n${disabledKey}\n`);
    startup.initialize();
    assert.equal(startup.isEnabled(), false);
    assert.match(fs.readFileSync(startup.filename, "utf8"), /Exec=\/old\/tron/);
    startup.setEnabled(true);
    assert.equal(startup.isEnabled(), true);
  });
}

test("Exec escapes reserved characters without invoking a shell", () => {
  assert.equal(quoteDesktopExecutable('/apps/Tron "test" $HOME `id` \\ 100%.AppImage'),
    String.raw`"/apps/Tron \\"test\\" \\$HOME \\` + '`id' + String.raw`\\` + '`' + String.raw` \\\\ 100%%.AppImage"`);
  for (const executable of ["relative/tron", "/apps/tron\nHidden=false", "/apps/tron\0", "/apps/tron=debug"]) {
    assert.throws(() => quoteDesktopExecutable(executable), /valid absolute executable path/);
  }
});

test("Linux honors desktop environment restrictions in an existing startup entry", t => {
  const { settings } = fixture(t);
  const startup = createLinuxAutostart({ ...settings, env: { XDG_CURRENT_DESKTOP: "ubuntu:GNOME" } });
  startup.initialize();
  fs.appendFileSync(startup.filename, "OnlyShowIn=KDE;\n");
  assert.equal(startup.isEnabled(), false);
  startup.setEnabled(true);
  fs.appendFileSync(startup.filename, "OnlyShowIn=GNOME;\n");
  assert.equal(startup.isEnabled(), true);
  fs.appendFileSync(startup.filename, "NotShowIn=GNOME;\n");
  assert.equal(startup.isEnabled(), false);
});
