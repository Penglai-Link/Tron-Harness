const fs = require("node:fs");
const path = require("node:path");

function acknowledgementPath(dataDir, version) {
  // Keep each version independent, including when users roll back and upgrade again.
  return path.join(dataDir, "release-notes", `${encodeURIComponent(version)}.ack`);
}

function isAcknowledged(dataDir, version) {
  try {
    return fs.readFileSync(acknowledgementPath(dataDir, version), "utf8") === version;
  } catch (error) {
    if (error.code === "ENOENT") return false;
    throw error;
  }
}

function acknowledge(dataDir, version) {
  const destination = acknowledgementPath(dataDir, version);
  fs.mkdirSync(path.dirname(destination), { recursive: true });
  const temporary = `${destination}.tmp`;
  fs.writeFileSync(temporary, version, "utf8");
  fs.renameSync(temporary, destination);
}

module.exports = { isAcknowledged, acknowledge };
