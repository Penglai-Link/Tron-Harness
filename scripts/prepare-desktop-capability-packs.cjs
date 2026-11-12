const path = require("node:path");
const { spawnSync } = require("node:child_process");

const repository = path.resolve(__dirname, "..");
const directory = process.argv[2];
if (!directory) throw new Error("缺少桌面能力包资源目录");
const cargo = process.env.TRON_COMPUTER_USE_CARGO || "cargo";
const cargoArgs = ["run", "--locked", "--manifest-path", "capability-packs/computer-use/packager/Cargo.toml", "--"];

function run(program, args) {
  const result = spawnSync(program, args, { cwd: repository, stdio: "inherit", env: process.env });
  if (result.error) throw result.error;
  if (result.signal || result.status === null) throw new Error(`${program} 被中断`);
  return result.status;
}

// Let the checker exit before building: Windows cannot replace a running exe.
const checked = run(cargo, [...cargoArgs, "--check-desktop-cache"]);
if (checked === 2) {
  console.log("执行 make capability-computer-use 构建当前平台能力包");
  const built = run(process.env.TRON_COMPUTER_USE_MAKE || "make", [
    "capability-computer-use",
    "VERSION=",
    `SKIP_TESTS=${process.env.TRON_COMPUTER_USE_SKIP_TESTS || "false"}`,
    "COMPUTER_USE_WINDOWS_PACKAGE=",
  ]);
  if (built !== 0) process.exit(built);
} else if (checked !== 0) {
  process.exit(checked);
}
process.exit(run(cargo, [...cargoArgs, "--desktop-dir", directory]));
