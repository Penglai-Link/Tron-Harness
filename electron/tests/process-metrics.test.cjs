const { test } = require("node:test");
const assert = require("node:assert/strict");
const { collectProcessMetrics } = require("../main/process-metrics.cjs");
test("sums every Electron process once, converting KiB to bytes", () => {
  const metrics = ["Browser", "Tab", "GPU", "Utility", "Zygote"].map((type, i) => ({ pid: i + 1, type, creationTime: 123, memory: { workingSetSize: 1024 } }));
  const result = collectProcessMetrics({ getAppMetrics: () => [...metrics, metrics[1]] });
  assert.equal(result.memoryBytes, 5 * 1048576);
  assert.equal(result.processes.length, 5);
  assert.ok(result.processes.every(p => p.parentPid === null));
  assert.equal(result.processes[2].name, "Electron · GPU");
});
test("missing process memory leaves the total unavailable instead of undercounting", () => {
  const result = collectProcessMetrics({ getAppMetrics: () => [{ pid: 1, type: "Browser", memory: { workingSetSize: 1024 } }, { pid: 2, type: "Utility" }] });
  assert.equal(result.memoryBytes, null);
  assert.equal(result.processes[1].memoryBytes, null);
  assert.equal(collectProcessMetrics({ getAppMetrics: () => [] }).memoryBytes, null);
});
