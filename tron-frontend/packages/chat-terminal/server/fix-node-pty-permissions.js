import { chmod } from "node:fs/promises";
import path from "node:path";

if (process.platform === "darwin") {
  await Promise.all(
    ["darwin-arm64", "darwin-x64"].map(async (platform) => {
      const helper = path.join(
        process.cwd(),
        "node_modules",
        "node-pty",
        "prebuilds",
        platform,
        "spawn-helper",
      );

      try {
        await chmod(helper, 0o755);
      } catch (error) {
        if (error?.code !== "ENOENT") throw error;
      }
    }),
  );
}
