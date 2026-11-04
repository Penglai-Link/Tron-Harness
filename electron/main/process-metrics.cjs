// Electron owns this process inventory; do not scan other desktop applications.
function collectProcessMetrics(app) {
  const processes = [...new Map(app.getAppMetrics().map(metric => [metric.pid, metric])).values()]
    .map(metric => {
      const kib = metric.memory?.workingSetSize;
      return {
        pid: metric.pid,
        parentPid: null, // app.getAppMetrics does not expose OS parent relationships.
        name: `Electron · ${metric.type}${metric.name ? ` · ${metric.name}` : ""}`,
        memoryBytes: Number.isFinite(kib) && kib >= 0 ? kib * 1024 : null,
        startedAt: metric.creationTime,
      };
    });
  return {
    sampledAt: Date.now(),
    memoryBytes: processes.length && processes.every(p => p.memoryBytes !== null)
      ? processes.reduce((sum, p) => sum + p.memoryBytes, 0) : null,
    processes,
  };
}
module.exports = { collectProcessMetrics };
