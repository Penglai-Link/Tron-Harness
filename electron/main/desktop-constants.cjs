const path = require("node:path");

module.exports = {
  APP_ID: "com.tron.desktop",
  STARTUP_ARG: "--tron-startup",
  ICON_PATH: path.resolve(__dirname, "..", "assets", "tron.png")
};
