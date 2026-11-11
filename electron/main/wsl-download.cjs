const { Readable } = require("node:stream");

function secureDownloadUrl(value, previous) {
  const url = new URL(value, previous);
  const localHttp = url.protocol === "http:" && ["localhost", "127.0.0.1", "[::1]"].includes(url.hostname);
  if ((url.protocol !== "https:" && !localHttp) || (previous?.protocol === "https:" && url.protocol !== "https:") || url.username || url.password) {
    throw new Error("wsl-insecure-download-url");
  }
  return url;
}

// Electron net.fetch has an empty/incorrect Response.url. Track the real URL
// through ClientRequest redirect events, validating before following each hop.
function fetchWslDownload(net, value, { signal } = {}) {
  return new Promise((resolve, reject) => {
    let current = secureDownloadUrl(value);
    signal?.throwIfAborted();
    const request = net.request({ url: current.href, method: "GET", redirect: "manual" });
    let redirects = 0;
    let incoming;
    const fail = (error) => {
      reject(error);
      incoming?.destroy(error);
      request.abort();
    };
    const abort = () => fail(signal.reason ?? new Error("wsl-package-download-aborted"));
    request.on("error", fail);
    request.on("abort", () => reject(new Error("wsl-package-download-aborted")));
    request.once("close", () => signal?.removeEventListener("abort", abort));
    signal?.addEventListener("abort", abort, { once: true });
    if (signal?.aborted) { abort(); return; }
    request.on("redirect", (_status, _method, location) => {
      try {
        if (++redirects > 10) throw new Error("wsl-package-too-many-redirects");
        current = secureDownloadUrl(location, current);
        request.followRedirect();
      } catch (error) { fail(error); }
    });
    request.once("response", (response) => {
      incoming = response;
      let ended = false;
      response.once("end", () => { ended = true; });
      response.once("close", () => { if (!ended) request.abort(); });
      response.on("error", reject);
      response.once("aborted", () => response.destroy(new Error("wsl-package-download-aborted")));
      try {
        const headers = new Headers();
        for (const [name, values] of Object.entries(response.headers)) {
          for (const value of Array.isArray(values) ? values : [values]) headers.append(name, value);
        }
        const result = new Response(Readable.toWeb(response), { status: response.statusCode, headers });
        Object.defineProperty(result, "url", { value: current.href });
        Object.defineProperty(result, "redirected", { value: redirects > 0 });
        resolve(result);
      } catch (error) { fail(error); }
    });
    request.end();
  });
}

module.exports = { fetchWslDownload, secureDownloadUrl };
