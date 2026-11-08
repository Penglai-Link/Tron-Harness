import { access, lstat, mkdir, readFile, realpath, symlink, unlink } from "node:fs/promises";
import { resolve, dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

// Local multi-repository development. Published builds must use released packages.
// Never copy shared source into Tron or silently select a different checkout.
const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const dependencies = [
  ["agent-framework", process.env.TRON_AGENT_FRAMEWORK_DIR, "Cargo.toml"],
];
const links = [];
for (const [name, configured, manifest] of dependencies) {
  if (!configured) throw new Error(`Set TRON_AGENT_FRAMEWORK_DIR to the intended public-project checkout`);
  const source = await realpath(resolve(configured));
  await access(join(source, manifest));
  if (name === "agent-framework") {
    const frameworkManifest = await readFile(join(source, "crates/framework/eliteforge-agent-framework/Cargo.toml"), "utf8");
    if (!/^name\s*=\s*"eliteforge-agent-framework"\s*$/m.test(frameworkManifest)) {
      throw new Error(`Not an Agent Framework checkout: ${source}`);
    }
  }
  const target = join(root, ".agent-deps", name);
  const current = await lstat(target).catch(error => {
    if (error.code !== "ENOENT") throw error;
    return null;
  });
  if (current) {
    if (!current.isSymbolicLink()) throw new Error(`Refusing to replace non-link path: ${target}`);
  }
  links.push({ name, source, target, current });
}
// Validate the repository before modifying its link.
await mkdir(join(root, ".agent-deps"), { recursive: true });
for (const { name, source, target, current } of links) {
  if (current && await realpath(target).catch(() => null) === source) {
    console.log(`${name}: ${source}`);
    continue;
  }
  if (current) await unlink(target);
  await symlink(source, target, process.platform === "win32" ? "junction" : "dir");
  console.log(`${name}: ${source}`);
}
