precision highp float;
uniform sampler2D sceneTexture;
uniform sampler2D bloomTexture;
uniform vec2 resolution;
uniform vec2 direction;
uniform float mode;
vec3 decode(vec3 value) { value *= value; return value / max(vec3(0.015), 1.0 - value); }
vec3 encode(vec3 value) { return sqrt(value / (1.0 + value)); }
vec3 film(vec3 x) { return clamp((x*(2.51*x+0.03))/(x*(2.43*x+0.59)+0.14),0.0,1.0); }
void main() {
  vec2 uv = gl_FragCoord.xy / resolution;
  if (mode < 1.5) {
    vec3 sum = vec3(0.0); float weightSum = 0.0;
    for (int i = -6; i <= 6; i++) {
      float offset = float(i);
      float weight = exp(-offset * offset / 12.0);
      vec3 sampleLight = decode(texture2D(sceneTexture, uv + direction * offset).rgb);
      if (mode < 0.5) sampleLight = max(sampleLight - 0.8, 0.0);
      sum += sampleLight * weight; weightSum += weight;
    }
    gl_FragColor = vec4(encode(sum / weightSum), 1.0);
  } else {
    vec3 light = decode(texture2D(sceneTexture, uv).rgb);
    vec3 glow = decode(texture2D(bloomTexture, uv).rgb);
    vec3 halo = vec3(0.0);
    for (int i = 0; i < 8; i++) {
      float angle = float(i) * 0.785398;
      halo += decode(texture2D(bloomTexture, uv + vec2(cos(angle),sin(angle)) * 34.0 / resolution).rgb);
    }
    light += glow * 0.45 + halo * 0.035;
    // Wide highlights retain a warm corona; the brightest knots roll off to white.
    vec3 result = pow(film(light * 0.85), vec3(1.0 / 2.2));
    float vignette = 1.0 - 0.24 * smoothstep(0.35, 0.95, length(uv - 0.5));
    gl_FragColor = vec4(result * vignette, 1.0);
  }
}
