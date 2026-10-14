const fs = require("node:fs");
const path = require("node:path");

// Share the application's daily append-only stream so existing log queries,
// retention and cursor pagination include desktop events as well as backend events.
function createApplicationLogger(directory) {
  return (level, message, attributes = {}) => {
    try {
      const timestamp = new Date().toISOString();
      fs.mkdirSync(directory, { recursive: true });
      fs.appendFileSync(path.join(directory, `tron-backend.${timestamp.slice(0, 10)}.jsonl`),
        JSON.stringify({ ...attributes, timestamp, level, target: "tron.desktop.updater", message }) + "\n", "utf8");
    } catch (error) {
      // Logging must not change the result of an update operation.
      console.warn("Could not persist desktop update log:", error.message);
    }
  };
}
module.exports = { createApplicationLogger };
