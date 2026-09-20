const fs = require("node:fs");
const path = require("node:path");

const outputPath = process.argv[2];
if (!outputPath) {
  throw new Error("usage: node write-update-config.cjs <output-path>");
}

const versionServerUrl = String(process.env.TRON_VERSION_SERVER_URL || "").trim();
fs.mkdirSync(path.dirname(outputPath), { recursive: true });
fs.writeFileSync(outputPath, `${JSON.stringify({ versionServerUrl }, null, 2)}\n`, "utf8");
