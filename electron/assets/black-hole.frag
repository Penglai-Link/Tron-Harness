#if __VERSION__ < 300
#extension GL_OES_standard_derivatives : enable
#endif
precision highp float;
uniform vec2 resolution;
uniform vec2 pointer;
uniform float time;
uniform float plunge;
uniform float approach;

float hash(vec2 p) {
  return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453123);
}
float noise(vec2 p) {
  vec2 cell = floor(p), f = fract(p);
  f = f * f * (3.0 - 2.0 * f);
  return mix(mix(hash(cell), hash(cell + vec2(1, 0)), f.x),
             mix(hash(cell + vec2(0, 1)), hash(cell + 1.0), f.x), f.y);
}
float turbulence(vec2 p) {
  return noise(p) * 0.57 + noise(p * 2.03 + 7.3) * 0.28
       + noise(p * 4.07 + 21.1) * 0.15;
}
vec3 starfield(vec3 direction) {
  vec2 sky = vec2(atan(direction.x, direction.z), asin(clamp(direction.y, -1.0, 1.0)));
  vec3 light = vec3(0.0);
  for (int layer = 0; layer < 2; layer++) {
    vec2 grid = sky * (190.0 + float(layer) * 110.0);
    vec2 cell = floor(grid);
    float seed = hash(cell + float(layer) * 37.0);
    vec2 offset = vec2(hash(cell + 3.4), hash(cell + 9.1)) * 0.7 + 0.15;
    float distance = length(fract(grid) - offset);
    float star = exp(-distance * distance * 720.0) * step(0.9955, seed);
    light += mix(vec3(0.52, 0.65, 0.83), vec3(0.92, 0.85, 0.73), seed) * star * 0.62;
  }
  return light;
}
// Periodic gradient noise has no rectangular value-noise plateaus. Each octave
// fades out before its wavelength drops below the projected pixel footprint.
vec2 gradient(vec2 cell, float period) {
  cell.y = mod(cell.y, period);
  vec3 h = fract(vec3(cell.xyx) * vec3(0.1031, 0.1030, 0.0973));
  h += dot(h, h.yzx + 33.33);
  vec2 g = fract((h.xx + h.yz) * h.zy) * 2.0 - 1.0;
  return g * inversesqrt(max(dot(g, g), 0.001));
}
float silkNoise(vec2 p, float period) {
  vec2 i = floor(p), f = fract(p);
  vec2 w = f*f*f*(f*(f*6.0-15.0)+10.0);
  float a = dot(gradient(i, period), f);
  float b = dot(gradient(i+vec2(1,0), period), f-vec2(1,0));
  float c = dot(gradient(i+vec2(0,1), period), f-vec2(0,1));
  float d = dot(gradient(i+1.0, period), f-1.0);
  return mix(mix(a,b,w.x),mix(c,d,w.x),w.y);
}
float filteredSilk(vec2 p, vec2 footprint, float period) {
  float sum = 0.0, amplitude = 0.52;
  for (int octave = 0; octave < 5; octave++) {
    float coverage = 1.0 - smoothstep(0.25, 0.8, max(footprint.x, footprint.y));
    sum += silkNoise(p, period) * amplitude * coverage;
    p *= 2.0; footprint *= 2.0; period *= 2.0; amplitude *= 0.48;
  }
  return 0.5 + sum * 0.72;
}
vec2 diskMaterial(float radius, float angle, float age, vec2 footprint) {
  float speed = 0.24 / pow(max(radius / 3.0, 1.0), 1.5);
  float orbit = angle - age * speed;
  // Include differential shear in the texture footprint, rather than filtering
  // only the undeformed disk coordinates.
  // Resolve broader strands when the whole disk is distant; fine filaments
  // remain in the near view instead of aliasing or averaging into featureless fog.
  float radialFrequency = mix(2.8, 12.0, approach);
  vec2 uv = vec2(radius * radialFrequency, orbit * 2.546479);
  vec2 fw = vec2(footprint.x * radialFrequency,
    (footprint.y + footprint.x * age * speed * 1.5 / max(radius, 3.0)) * 2.546479);
  float currents = filteredSilk(uv * vec2(0.12, 0.5), fw * vec2(0.12, 0.5), 8.0);
  // Long azimuthal correlation forms irregular continuous strands instead of
  // short noise islands or evenly spaced mathematical rings.
  uv.x += (currents - 0.5) * 1.8;
  float strands = filteredSilk(uv, fw * 1.35, 16.0);
  float fine = filteredSilk(uv * 2.0, fw * 2.7, 32.0);
  float density = 0.025 + pow(strands, 4.0) * 2.6 + pow(fine, 4.0) * 0.12;
  float flecks = filteredSilk(uv * vec2(1.25, 16.0), fw * vec2(1.7, 21.6), 256.0);
  float knots = pow(smoothstep(0.55, 0.82, flecks), 3.0);
  return vec2(density, knots);
}
vec3 diskLight(vec3 hit, vec3 direction, vec2 footprint) {
  float radius = length(hit.xz);
  float inner = smoothstep(2.55, 2.95, radius);
  float outerRadius = mix(mix(14.0, 8.5, approach), mix(14.0, 30.0, approach), smoothstep(-1.0, 4.0, hit.z));
  float outer = 1.0 - smoothstep(outerRadius * 0.65, outerRadius, radius);
  float angle = atan(hit.z, hit.x);
  float age = mod(time, 20.0);
  vec2 matter = mix(diskMaterial(radius, angle, age, footprint),
                    diskMaterial(radius, angle, mod(time + 10.0, 20.0), footprint), abs(age / 10.0 - 1.0));
  // At a distance, filtered filaments occupy less than a pixel. Preserve the
  // outer disk's luminous envelope instead of leaving only the photon arc.
  float heat = pow(3.0 / max(radius, 3.0), mix(1.8, 3.7, approach));
  float approaching = dot(direction, normalize(vec3(-hit.z, 0.0, hit.x)));
  float doppler = pow(clamp(1.0 + approaching * 0.55, 0.45, 1.55), 2.0);
  float wake = exp(-pow((hit.x + hit.z * 0.04) / (0.38 + radius * 0.07), 2.0));
  wake *= smoothstep(1.0, 4.5, hit.z);
  float flares = matter.y * wake * pow(3.0 / max(radius, 3.0), 0.6) * 18.0;
  vec3 color = mix(vec3(1.0, 0.29, 0.055), vec3(1.0, 0.72, 0.30), pow(heat, 0.25));
  return color * (matter.x * heat * 5.0 * doppler + flares) * inner * outer;
}
vec3 gravity(vec3 position, float angularMomentum) {
  float r2 = dot(position, position);
  return -1.5 * angularMomentum * position / (r2 * r2 * sqrt(r2));
}
void main() {
  float aspect = resolution.x / resolution.y;
  vec2 screen = (gl_FragCoord.xy / resolution - 0.5) * 2.0;
  screen.x *= aspect;
  float drive = plunge * plunge * (3.0 - 2.0 * plunge);
  vec2 center = aspect > 1.0 ? vec2(aspect * 0.30, 0.06) : vec2(0.0, 0.46);
  vec2 p = screen - center * (1.0 - drive);
  // Slight roll gives the disk an ascending silhouette rather than a horizontal stripe.
  p = mat2(0.9272, 0.3746, -0.3746, 0.9272) * p;
  float distanceToHole = mix(42.0, 14.5, approach);
  vec3 origin = vec3(pointer.x * 0.12 * (1.0 - drive),
                     mix(distanceToHole * 0.16 + pointer.y * 0.10, 0.025, drive), mix(distanceToHole, 0.7, drive));
  vec3 forward = normalize(-origin);
  vec3 right = normalize(cross(forward, vec3(0, 1, 0)));
  vec3 up = cross(right, forward);
  vec3 direction = normalize(forward * 2.15 + right * p.x + up * p.y);
  vec3 position = origin;
  vec3 angular = cross(origin, direction);
  float momentum = dot(angular, angular);
  vec3 light = vec3(0.0);
  float visibility = 1.0;
  bool captured = false;
  int crossings = 0;
  vec3 diskHit = vec3(0.0, 0.0, 1.0), diskDirection = direction;
  bool hitDisk = false;
  // Integrate ray curvature; plane interpolation keeps the disk sharp at every quality level.
  for (int i = 0; i < 220; i++) {
    float radius = length(position);
    if (radius < 1.02) { captured = true; break; }
    if (radius > 35.0 && dot(position, direction) > 0.0) break;
    float stepSize = clamp((radius - 1.0) * 0.105, 0.035, 1.4);
    vec3 acceleration = gravity(position, momentum);
    vec3 next = position + direction * stepSize + acceleration * stepSize * stepSize * 0.5;
    vec3 nextDirection = direction + (acceleration + gravity(next, momentum)) * (0.5 * stepSize);
    if (position.y * next.y < 0.0) {
      crossings++;
      // Higher-order disk images collapse to a distracting one-pixel circle.
      // Treat the second crossing as occulted by the optically thick disk.
      if (crossings > 1) { captured = true; break; }
      diskHit = mix(position, next, position.y / (position.y - next.y));
      diskDirection = normalize(direction);
      hitDisk = true;
      // The optically thick disk terminates the ray. Avoid tracing an invisible
      // continuation, preserving budget for native-resolution material detail.
      float diskRadius = length(diskHit.xz);
      if (diskRadius > 2.95 && diskRadius < 30.0) break;
    }
    position = next;
    direction = nextDirection;
  }
  // Derivatives are evaluated outside divergent ray-marching branches.
  vec2 diskPosition = diskHit.xz;
  vec2 dx = dFdx(diskPosition), dy = dFdy(diskPosition);
  float diskR2 = max(dot(diskPosition, diskPosition), 0.001);
  // Differentiate atan analytically: its -pi/pi wrap is not a texture seam.
  vec2 footprint = vec2(
    (abs(dot(diskPosition, dx)) + abs(dot(diskPosition, dy))) / sqrt(diskR2),
    (abs(diskPosition.x * dx.y - diskPosition.y * dx.x)
      + abs(diskPosition.x * dy.y - diskPosition.y * dy.x)) / diskR2);
  if (hitDisk) {
    vec3 emission = diskLight(diskHit, diskDirection, footprint);
    light += emission;
    visibility = 1.0 - smoothstep(0.001, 0.025, length(emission));
  }
  if (!captured) light += starfield(normalize(direction)) * visibility;
  // Dust behind the disk adds cold depth while the horizon itself stays black.
  float dust = turbulence(screen * 1.8 + vec2(12.0, 4.0));
  light += vec3(0.0016, 0.0032, 0.0040) * dust * (captured ? 0.0 : 1.0);
  light *= 1.0 - smoothstep(0.84, 1.0, plunge);
  // Encode HDR into a filterable RGBA8 target. Both hosts use the same bloom
  // pipeline and decode before filtering; no float-texture extension is needed.
  light = min(light, vec3(64.0));
  gl_FragColor = vec4(sqrt(light / (1.0 + light)), 1.0);
}
