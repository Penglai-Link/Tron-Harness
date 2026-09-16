import { readFile, writeFile } from "node:fs/promises";
import process from "node:process";

const root = new URL("../", import.meta.url);
const versionFile = new URL("version.properties", root);
const versionPattern = /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/;

const readRevision = async () => {
  const content = await readFile(versionFile, "utf8");
  const match = content.match(/^revision\s*=\s*(\S+)\s*$/m);
  if (!match) throw new Error("version.properties must define revision=<semver>");
  return match[1];
};

const checkOnly = process.argv[2] === "--check";
const requestedRevision = checkOnly ? undefined : process.argv[2];
const revision = requestedRevision ?? await readRevision();
if (!versionPattern.test(revision)) {
  throw new Error(`Invalid revision '${revision}'. Expected semantic version x.y.z[-qualifier].`);
}

const files = [
  "electron/package.json",
  "tron-frontend/package.json",
  "tron-frontend/packages/chat-internal-adapter/package.json",
];
for (const relativePath of files) {
  const url = new URL(relativePath, root);
  const packageJson = JSON.parse(await readFile(url, "utf8"));
  if (checkOnly) {
    if (packageJson.version !== revision) throw new Error(`${relativePath} is ${packageJson.version}, expected ${revision}`);
  } else {
    packageJson.version = revision;
    await writeFile(url, `${JSON.stringify(packageJson, null, 2)}\n`);
  }
}

const replaceFirst = async (relativePath, pattern, replacement) => {
  const url = new URL(relativePath, root);
  const content = await readFile(url, "utf8");
  if (!pattern.test(content)) throw new Error(`Could not find version in ${relativePath}`);
  const updated = content.replace(pattern, replacement);
  if (checkOnly) {
    if (updated !== content) throw new Error(`${relativePath} does not use revision ${revision}`);
  } else {
    await writeFile(url, updated);
  }
};
if (!checkOnly) {
  await writeFile(versionFile, `# Single source of truth for the Tron product version.\n# Keep this compatible with Maven's ${"${revision}"} convention.\nrevision=${revision}\n`);
}
await replaceFirst("tron-backend/Cargo.toml", /^version\s*=\s*"[^"]+"/m, `version = "${revision}"`);
await replaceFirst("openapi.yaml", /^(\s+version:\s*)[^\s]+/m, `$1${revision}`);

console.log(checkOnly ? `Tron revision is consistent at ${revision}` : `Tron revision synchronized to ${revision}`);
