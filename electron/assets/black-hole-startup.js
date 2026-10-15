// The startup page exists before the backend and React. Use the same GLSL asset
// with a minimal WebGL host; no network, backend, Node integration or CDN needed.
function startBlackHole(canvas, fragment, postFragment) {
  const gl = canvas.getContext("webgl", {
    alpha: false,
    antialias: false,
    powerPreference: "high-performance",
  });
  if (!gl || !gl.getExtension("OES_standard_derivatives")) return;
  let program,
    buffer,
    frame = 0,
    disposed = false,
    lost = false,
    last = 0,
    slow = 0,
    fast = 0;
  const startedAt = performance.now();
  let scale = Math.min(devicePixelRatio || 1, 1.25);
  let locations, postProgram, postLocations;
  const targets = [];
  const shaders = [];
  function init() {
    function shader(type, source) {
      const result = gl.createShader(type);
      shaders.push(result);
      gl.shaderSource(result, source);
      gl.compileShader(result);
      if (!gl.getShaderParameter(result, gl.COMPILE_STATUS))
        throw new Error(gl.getShaderInfoLog(result));
      return result;
    }
    function link(source) {
      const result = gl.createProgram();
      gl.attachShader(
        result,
        shader(
          gl.VERTEX_SHADER,
          "attribute vec2 position;void main(){gl_Position=vec4(position,0.,1.);}"
        )
      );
      gl.attachShader(result, shader(gl.FRAGMENT_SHADER, source));
      gl.bindAttribLocation(result, 0, "position");
      gl.linkProgram(result);
      if (!gl.getProgramParameter(result, gl.LINK_STATUS))
        throw new Error(gl.getProgramInfoLog(result));
      return result;
    }
    program = link(fragment);
    postProgram = link(postFragment);
    postLocations = Object.fromEntries(
      ["resolution", "direction", "mode", "lightTheme", "sceneTexture", "bloomTexture"].map(
        (key) => [key, gl.getUniformLocation(postProgram, key)]
      )
    );
    gl.useProgram(program);
    buffer = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
    gl.bufferData(
      gl.ARRAY_BUFFER,
      new Float32Array([-1, -1, 1, -1, -1, 1, -1, 1, 1, -1, 1, 1]),
      gl.STATIC_DRAW
    );
    const position = gl.getAttribLocation(program, "position");
    gl.enableVertexAttribArray(position);
    gl.vertexAttribPointer(position, 2, gl.FLOAT, false, 0, 0);
    locations = Object.fromEntries(
      ["resolution", "pointer", "time", "plunge", "approach", "lightTheme"].map((key) => [
        key,
        gl.getUniformLocation(program, key),
      ])
    );
    gl.uniform2f(locations.pointer, 0, 0);
    gl.uniform1f(locations.plunge, 0);
    gl.uniform1f(locations.approach, 0);
    for (let i = 0; i < 3; i++) {
      const texture = gl.createTexture(),
        framebuffer = gl.createFramebuffer();
      gl.bindTexture(gl.TEXTURE_2D, texture);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
      gl.bindFramebuffer(gl.FRAMEBUFFER, framebuffer);
      gl.framebufferTexture2D(
        gl.FRAMEBUFFER,
        gl.COLOR_ATTACHMENT0,
        gl.TEXTURE_2D,
        texture,
        0
      );
      targets.push({ texture, framebuffer, width: 1, height: 1 });
    }
    resize();
  }
  function resize() {
    const ratio = Math.min(
      scale,
      Math.sqrt(2073600 / Math.max(1, innerWidth * innerHeight))
    );
    canvas.width = Math.max(1, Math.round(innerWidth * ratio));
    canvas.height = Math.max(1, Math.round(innerHeight * ratio));
    targets.forEach((target, index) => {
      target.width = Math.max(1, Math.round(canvas.width / (index ? 4 : 1)));
      target.height = Math.max(1, Math.round(canvas.height / (index ? 4 : 1)));
      gl.bindTexture(gl.TEXTURE_2D, target.texture);
      gl.texImage2D(
        gl.TEXTURE_2D,
        0,
        gl.RGBA,
        target.width,
        target.height,
        0,
        gl.RGBA,
        gl.UNSIGNED_BYTE,
        null
      );
      gl.bindFramebuffer(gl.FRAMEBUFFER, target.framebuffer);
      if (gl.checkFramebufferStatus(gl.FRAMEBUFFER) !== gl.FRAMEBUFFER_COMPLETE)
        throw new Error("Incomplete black hole render target");
    });
  }
  function draw(now) {
    if (disposed || lost || document.hidden) return;
    if (last && now - startedAt > 3000) {
      const duration = now - last;
      slow = duration > 45 ? slow + 1 : Math.max(0, slow - 2);
      fast = duration < 24 ? fast + 1 : 0;
      if (slow > 120 && scale > 1) {
        scale = Math.max(1, scale - 0.25);
        slow = 0;
        resize();
      } else if (fast > 600 && scale < Math.min(devicePixelRatio || 1, 1.25)) {
        scale = Math.min(scale + 0.25, devicePixelRatio || 1, 1.25);
        fast = 0;
        resize();
      }
    }
    last = now;
    gl.useProgram(program);
    gl.bindFramebuffer(gl.FRAMEBUFFER, targets[0].framebuffer);
    gl.viewport(0, 0, canvas.width, canvas.height);
    gl.uniform2f(locations.resolution, canvas.width, canvas.height);
    gl.uniform1f(locations.lightTheme, document.documentElement.dataset.theme === "light" ? 1 : 0);
    gl.uniform1f(locations.time, (Date.now() % 3600000) / 1000);
    gl.drawArrays(gl.TRIANGLES, 0, 6);
    gl.useProgram(postProgram);
    gl.uniform1f(postLocations.lightTheme, document.documentElement.dataset.theme === "light" ? 1 : 0);
    gl.uniform1i(postLocations.sceneTexture, 0);
    gl.uniform1i(postLocations.bloomTexture, 1);
    for (let pass = 0; pass < 3; pass++) {
      const target = pass < 2 ? targets[pass + 1] : null;
      gl.bindFramebuffer(gl.FRAMEBUFFER, target ? target.framebuffer : null);
      const width = target ? target.width : canvas.width,
        height = target ? target.height : canvas.height;
      gl.viewport(0, 0, width, height);
      gl.uniform2f(postLocations.resolution, width, height);
      gl.uniform1f(postLocations.mode, pass);
      gl.uniform2f(
        postLocations.direction,
        pass === 0 ? 4 / canvas.width : 0,
        pass === 1 ? 4 / canvas.height : 0
      );
      gl.activeTexture(gl.TEXTURE0);
      gl.bindTexture(gl.TEXTURE_2D, targets[pass === 1 ? 1 : 0].texture);
      gl.activeTexture(gl.TEXTURE1);
      gl.bindTexture(gl.TEXTURE_2D, pass === 2 ? targets[2].texture : null);
      gl.drawArrays(gl.TRIANGLES, 0, 6);
    }
    gl.activeTexture(gl.TEXTURE0);
    canvas.classList.add("black-hole-ready");
    frame = requestAnimationFrame(draw);
  }
  function visibility() {
    cancelAnimationFrame(frame);
    last = 0;
    if (!document.hidden && !lost) frame = requestAnimationFrame(draw);
  }
  function release() {
    shaders.splice(0).forEach((s) => gl.deleteShader(s));
    targets.splice(0).forEach((target) => {
      gl.deleteTexture(target.texture);
      gl.deleteFramebuffer(target.framebuffer);
    });
    if (postProgram) gl.deleteProgram(postProgram);
    if (program) gl.deleteProgram(program);
    if (buffer) gl.deleteBuffer(buffer);
  }
  function loss(event) {
    event.preventDefault();
    lost = true;
    cancelAnimationFrame(frame);
    canvas.classList.remove("black-hole-ready");
  }
  function restore() {
    try {
      release();
      init();
      lost = false;
      visibility();
    } catch {
      /* keep animated CSS fallback */
    }
  }
  function dispose() {
    disposed = true;
    cancelAnimationFrame(frame);
    release();
    removeEventListener("resize", resize);
    document.removeEventListener("visibilitychange", visibility);
    canvas.removeEventListener("webglcontextlost", loss);
    canvas.removeEventListener("webglcontextrestored", restore);
  }
  try {
    init();
  } catch {
    release();
    return;
  }
  addEventListener("resize", resize);
  document.addEventListener("visibilitychange", visibility);
  canvas.addEventListener("webglcontextlost", loss);
  canvas.addEventListener("webglcontextrestored", restore);
  addEventListener("pagehide", dispose, { once: true });
  frame = requestAnimationFrame(draw);
}
