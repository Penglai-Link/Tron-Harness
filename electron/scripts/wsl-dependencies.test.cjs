const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const { spawnSync } = require("node:child_process");
const { test } = require("node:test");

const script = fs.readFileSync(path.join(__dirname, "../main/wsl/dependencies.sh"), "utf8").replace(/\r\n/g, "\n");
const command = process.platform === "win32" ? "wsl.exe" : "sh";
const args = process.platform === "win32" ? ["--exec", "sh", "-s"] : ["-s"];
const available = spawnSync(command, args, { input: "exit 0\n", encoding: "utf8", timeout: 30000, windowsHide: true });

function runScenario(mode) {
  // Shell functions isolate APT and dpkg: these cases never install packages
  // or contact a network, including when running in the user's default WSL.
  const input = `
fixture=$(mktemp -d)
trap 'rm -rf -- "$fixture"' EXIT
mode=${mode}
dpkg_query() {
    for argument do package=$argument; done
    if [ "$mode" = installed ] || [ -f "$fixture/installed" ]; then
        printf installed
    elif [ "$package" = git ]; then
        if [ "$mode" = partial ]; then printf unpacked; else return 1; fi
    else
        printf installed
    fi
}
apt_get() {
    printf '%s\\n' "$*" >> "$fixture/apt"
    [ "$DEBIAN_FRONTEND" = noninteractive ] || return 99
    case " $* " in
        *' update '*) [ "$mode" != update-failed ] ;;
        *' install '*)
            [ "$mode" != install-failed ] || return 1
            if [ "$mode" != verification-failed ]; then touch "$fixture/installed"; fi
            ;;
        *) return 98 ;;
    esac
}
alias dpkg-query=dpkg_query
alias apt-get=apt_get
bootstrap() (
${script}
)
bootstrap
result=$?
printf '\\nTRON_RESULT=%s\\n' "$result"
if [ -f "$fixture/apt" ]; then cat "$fixture/apt"; fi
`;
  const result = spawnSync(command, args, { input, encoding: "utf8", timeout: 30000, windowsHide: true });
  assert.ifError(result.error);
  assert.equal(result.status, 0, result.stderr);
  const exitCode = Number(result.stdout.match(/TRON_RESULT=(\d+)/)?.[1]);
  const calls = result.stdout.split(/\r?\n/).filter((line) => line.startsWith("-o "));
  return { exitCode, calls, stderr: result.stderr };
}

const options = { skip: available.error || available.status !== 0 ? "POSIX shell unavailable (Windows requires a default WSL distribution)" : false };

test("installed dependencies require no APT access, even with an offline source", options, () => {
  const result = runScenario("installed");
  assert.equal(result.exitCode, 0);
  assert.deepEqual(result.calls, []);
});

for (const mode of ["missing", "partial"]) {
  test(`${mode} dependency installs only the affected package`, options, () => {
    const result = runScenario(mode);
    assert.equal(result.exitCode, 0, result.stderr);
    assert.equal(result.calls.length, 2);
    assert.match(result.calls[0], /APT::Update::Error-Mode=any update$/);
    assert.match(result.calls[1], /install -y --no-install-recommends git$/);
    for (const call of result.calls) {
      assert.match(call, /Acquire::ForceIPv4=true/);
      assert.match(call, /Acquire::http::Timeout=15/);
      assert.match(call, /Acquire::https::Timeout=15/);
    }
  });
}

for (const [mode, count, message] of [
  ["update-failed", 1, "APT 索引更新"],
  ["install-failed", 2, "APT 依赖安装"],
  ["verification-failed", 2, "依赖安装后检查"]
]) {
  test(`${mode} stops bootstrap with an actionable dependency error`, options, () => {
    const result = runScenario(mode);
    assert.equal(result.exitCode, 1);
    assert.equal(result.calls.length, count);
    assert.match(result.stderr, /git/);
    assert.ok(result.stderr.includes(message), result.stderr);
    assert.ok(result.stderr.includes("软件源、网络或代理"), result.stderr);
  });
}
