import {
  useEffect,
  useRef,
  type CSSProperties,
} from "react";
import {
  Effect,
  EffectComposer,
  EffectPass,
  RenderPass,
} from "postprocessing";
import * as THREE from "three";

import { cn } from "@/shared/lib/utils";

import "./PixelBlast.css";

type PixelBlastVariant = "square" | "circle" | "triangle" | "diamond";

interface PixelBlastProps {
  antialias?: boolean;
  autoPauseOffscreen?: boolean;
  className?: string;
  color?: string;
  edgeFade?: number;
  enableRipples?: boolean;
  liquid?: boolean;
  liquidRadius?: number;
  liquidStrength?: number;
  liquidWobbleSpeed?: number;
  noiseAmount?: number;
  patternDensity?: number;
  patternScale?: number;
  pixelSize?: number;
  pixelSizeJitter?: number;
  rippleIntensityScale?: number;
  rippleSpeed?: number;
  rippleThickness?: number;
  speed?: number;
  style?: CSSProperties;
  transparent?: boolean;
  variant?: PixelBlastVariant;
}

interface TouchPoint {
  age: number;
  force: number;
  vx: number;
  vy: number;
  x: number;
  y: number;
}

interface TouchTexture {
  addTouch: (point: { x: number; y: number }) => void;
  radiusScale: number;
  size: number;
  texture: THREE.Texture;
  update: () => void;
}

interface ReinitConfig {
  antialias: boolean;
  liquid: boolean;
  noiseAmount: number;
}

interface PixelBlastUniforms extends Record<string, THREE.IUniform> {
  uClickPos: { value: THREE.Vector2[] };
  uClickTimes: { value: Float32Array };
  uColor: { value: THREE.Color };
  uDensity: { value: number };
  uEdgeFade: { value: number };
  uEnableRipples: { value: number };
  uPixelJitter: { value: number };
  uPixelSize: { value: number };
  uResolution: { value: THREE.Vector2 };
  uRippleIntensity: { value: number };
  uRippleSpeed: { value: number };
  uRippleThickness: { value: number };
  uScale: { value: number };
  uShapeType: { value: number };
  uTime: { value: number };
}

interface PixelBlastRuntime {
  camera: THREE.OrthographicCamera;
  clickIx: number;
  clock: THREE.Clock;
  composer?: EffectComposer;
  liquidEffect?: Effect;
  material: THREE.ShaderMaterial;
  quad?: THREE.Mesh<THREE.PlaneGeometry, THREE.ShaderMaterial>;
  raf?: number;
  renderer: THREE.WebGLRenderer;
  resizeObserver?: ResizeObserver;
  scene: THREE.Scene;
  touch?: TouchTexture;
  uniforms: PixelBlastUniforms;
}

const MAX_CLICKS = 10;

function createTouchTexture(): TouchTexture {
  const size = 64;
  const canvas = document.createElement("canvas");
  canvas.width = size;
  canvas.height = size;

  const context = canvas.getContext("2d");
  if (!context) throw new Error("2D context not available");

  context.fillStyle = "black";
  context.fillRect(0, 0, canvas.width, canvas.height);

  const texture = new THREE.Texture(canvas);
  texture.minFilter = THREE.LinearFilter;
  texture.magFilter = THREE.LinearFilter;
  texture.generateMipmaps = false;

  const trail: TouchPoint[] = [];
  let lastPoint: { x: number; y: number } | null = null;
  const maxAge = 64;
  let radius = 0.1 * size;
  const speed = 1 / maxAge;

  const clear = () => {
    context.fillStyle = "black";
    context.fillRect(0, 0, canvas.width, canvas.height);
  };

  const drawPoint = (point: TouchPoint) => {
    const position = { x: point.x * size, y: (1 - point.y) * size };
    let intensity = 1;
    const easeOutSine = (value: number) => Math.sin((value * Math.PI) / 2);
    const easeOutQuad = (value: number) => -value * (value - 2);

    if (point.age < maxAge * 0.3) {
      intensity = easeOutSine(point.age / (maxAge * 0.3));
    } else {
      intensity = easeOutQuad(1 - (point.age - maxAge * 0.3) / (maxAge * 0.7)) || 0;
    }

    intensity *= point.force;
    const color = `${((point.vx + 1) / 2) * 255}, ${((point.vy + 1) / 2) * 255}, ${intensity * 255}`;
    const offset = size * 5;

    context.shadowOffsetX = offset;
    context.shadowOffsetY = offset;
    context.shadowBlur = radius;
    context.shadowColor = `rgba(${color}, ${0.22 * intensity})`;
    context.beginPath();
    context.fillStyle = "rgba(255, 0, 0, 1)";
    context.arc(position.x - offset, position.y - offset, radius, 0, Math.PI * 2);
    context.fill();
  };

  return {
    addTouch: (point) => {
      let force = 0;
      let vx = 0;
      let vy = 0;

      if (lastPoint) {
        const dx = point.x - lastPoint.x;
        const dy = point.y - lastPoint.y;
        if (dx === 0 && dy === 0) return;

        const distanceSquared = dx * dx + dy * dy;
        const distance = Math.sqrt(distanceSquared);
        vx = dx / (distance || 1);
        vy = dy / (distance || 1);
        force = Math.min(distanceSquared * 10_000, 1);
      }

      lastPoint = { x: point.x, y: point.y };
      trail.push({ x: point.x, y: point.y, age: 0, force, vx, vy });
    },
    get radiusScale() {
      return radius / (0.1 * size);
    },
    set radiusScale(value: number) {
      radius = 0.1 * size * value;
    },
    size,
    texture,
    update: () => {
      clear();

      for (let index = trail.length - 1; index >= 0; index -= 1) {
        const point = trail[index];
        const force = point.force * speed * (1 - point.age / maxAge);
        point.x += point.vx * force;
        point.y += point.vy * force;
        point.age += 1;

        if (point.age > maxAge) {
          trail.splice(index, 1);
        }
      }

      trail.forEach(drawPoint);
      texture.needsUpdate = true;
    },
  };
}

function createLiquidEffect(texture: THREE.Texture, options?: { freq?: number; strength?: number }) {
  const fragment = `
    uniform sampler2D uTexture;
    uniform float uStrength;
    uniform float uTime;
    uniform float uFreq;

    void mainUv(inout vec2 uv) {
      vec4 tex = texture2D(uTexture, uv);
      float vx = tex.r * 2.0 - 1.0;
      float vy = tex.g * 2.0 - 1.0;
      float intensity = tex.b;

      float wave = 0.5 + 0.5 * sin(uTime * uFreq + intensity * 6.2831853);
      float amt = uStrength * intensity * wave;

      uv += vec2(vx, vy) * amt;
    }
  `;

  return new Effect("LiquidEffect", fragment, {
    uniforms: new Map<string, THREE.Uniform>([
      ["uTexture", new THREE.Uniform(texture)],
      ["uStrength", new THREE.Uniform(options?.strength ?? 0.025)],
      ["uTime", new THREE.Uniform(0)],
      ["uFreq", new THREE.Uniform(options?.freq ?? 4.5)],
    ]),
  });
}

const SHAPE_MAP: Record<PixelBlastVariant, number> = {
  circle: 1,
  diamond: 3,
  square: 0,
  triangle: 2,
};

const VERTEX_SRC = `
void main() {
  gl_Position = vec4(position, 1.0);
}
`;

const FRAGMENT_SRC = `
precision highp float;

uniform vec3  uColor;
uniform vec2  uResolution;
uniform float uTime;
uniform float uPixelSize;
uniform float uScale;
uniform float uDensity;
uniform float uPixelJitter;
uniform int   uEnableRipples;
uniform float uRippleSpeed;
uniform float uRippleThickness;
uniform float uRippleIntensity;
uniform float uEdgeFade;

uniform int   uShapeType;
const int SHAPE_SQUARE   = 0;
const int SHAPE_CIRCLE   = 1;
const int SHAPE_TRIANGLE = 2;
const int SHAPE_DIAMOND  = 3;

const int   MAX_CLICKS = 10;

uniform vec2  uClickPos  [MAX_CLICKS];
uniform float uClickTimes[MAX_CLICKS];

out vec4 fragColor;

float Bayer2(vec2 a) {
  a = floor(a);
  return fract(a.x / 2. + a.y * a.y * .75);
}
#define Bayer4(a) (Bayer2(.5*(a))*0.25 + Bayer2(a))
#define Bayer8(a) (Bayer4(.5*(a))*0.25 + Bayer2(a))

#define FBM_OCTAVES     5
#define FBM_LACUNARITY  1.25
#define FBM_GAIN        1.0

float hash11(float n){ return fract(sin(n)*43758.5453); }

float vnoise(vec3 p){
  vec3 ip = floor(p);
  vec3 fp = fract(p);
  float n000 = hash11(dot(ip + vec3(0.0,0.0,0.0), vec3(1.0,57.0,113.0)));
  float n100 = hash11(dot(ip + vec3(1.0,0.0,0.0), vec3(1.0,57.0,113.0)));
  float n010 = hash11(dot(ip + vec3(0.0,1.0,0.0), vec3(1.0,57.0,113.0)));
  float n110 = hash11(dot(ip + vec3(1.0,1.0,0.0), vec3(1.0,57.0,113.0)));
  float n001 = hash11(dot(ip + vec3(0.0,0.0,1.0), vec3(1.0,57.0,113.0)));
  float n101 = hash11(dot(ip + vec3(1.0,0.0,1.0), vec3(1.0,57.0,113.0)));
  float n011 = hash11(dot(ip + vec3(0.0,1.0,1.0), vec3(1.0,57.0,113.0)));
  float n111 = hash11(dot(ip + vec3(1.0,1.0,1.0), vec3(1.0,57.0,113.0)));
  vec3 w = fp*fp*fp*(fp*(fp*6.0-15.0)+10.0);
  float x00 = mix(n000, n100, w.x);
  float x10 = mix(n010, n110, w.x);
  float x01 = mix(n001, n101, w.x);
  float x11 = mix(n011, n111, w.x);
  float y0  = mix(x00, x10, w.y);
  float y1  = mix(x01, x11, w.y);
  return mix(y0, y1, w.z) * 2.0 - 1.0;
}

float fbm2(vec2 uv, float t){
  vec3 p = vec3(uv * uScale, t);
  float amp = 1.0;
  float freq = 1.0;
  float sum = 1.0;
  for (int i = 0; i < FBM_OCTAVES; ++i){
    sum  += amp * vnoise(p * freq);
    freq *= FBM_LACUNARITY;
    amp  *= FBM_GAIN;
  }
  return sum * 0.5 + 0.5;
}

float maskCircle(vec2 p, float cov){
  float r = sqrt(cov) * .25;
  float d = length(p - 0.5) - r;
  float aa = 0.5 * fwidth(d);
  return cov * (1.0 - smoothstep(-aa, aa, d * 2.0));
}

float maskTriangle(vec2 p, vec2 id, float cov){
  bool flip = mod(id.x + id.y, 2.0) > 0.5;
  if (flip) p.x = 1.0 - p.x;
  float r = sqrt(cov);
  float d  = p.y - r*(1.0 - p.x);
  float aa = fwidth(d);
  return cov * clamp(0.5 - d/aa, 0.0, 1.0);
}

float maskDiamond(vec2 p, float cov){
  float r = sqrt(cov) * 0.564;
  return step(abs(p.x - 0.49) + abs(p.y - 0.49), r);
}

void main(){
  float pixelSize = uPixelSize;
  vec2 fragCoord = gl_FragCoord.xy - uResolution * .5;
  float aspectRatio = uResolution.x / uResolution.y;

  vec2 pixelId = floor(fragCoord / pixelSize);
  vec2 pixelUV = fract(fragCoord / pixelSize);

  float cellPixelSize = 8.0 * pixelSize;
  vec2 cellId = floor(fragCoord / cellPixelSize);
  vec2 cellCoord = cellId * cellPixelSize;
  vec2 uv = cellCoord / uResolution * vec2(aspectRatio, 1.0);

  float base = fbm2(uv, uTime * 0.05);
  base = base * 0.5 - 0.65;

  float feed = base + (uDensity - 0.5) * 0.3;

  float speed = uRippleSpeed;
  float thickness = uRippleThickness;
  const float dampT = 1.0;
  const float dampR = 10.0;

  if (uEnableRipples == 1) {
    for (int i = 0; i < MAX_CLICKS; ++i){
      vec2 pos = uClickPos[i];
      if (pos.x < 0.0) continue;
      float cellPixelSize = 8.0 * pixelSize;
      vec2 cuv = (((pos - uResolution * .5 - cellPixelSize * .5) / (uResolution))) * vec2(aspectRatio, 1.0);
      float t = max(uTime - uClickTimes[i], 0.0);
      float r = distance(uv, cuv);
      float waveR = speed * t;
      float ring  = exp(-pow((r - waveR) / thickness, 2.0));
      float atten = exp(-dampT * t) * exp(-dampR * r);
      feed = max(feed, ring * atten * uRippleIntensity);
    }
  }

  float bayer = Bayer8(fragCoord / uPixelSize) - 0.5;
  float bw = step(0.5, feed + bayer);

  float h = fract(sin(dot(floor(fragCoord / uPixelSize), vec2(127.1, 311.7))) * 43758.5453);
  float jitterScale = 1.0 + (h - 0.5) * uPixelJitter;
  float coverage = bw * jitterScale;
  float M;
  if      (uShapeType == SHAPE_CIRCLE)   M = maskCircle(pixelUV, coverage);
  else if (uShapeType == SHAPE_TRIANGLE) M = maskTriangle(pixelUV, pixelId, coverage);
  else if (uShapeType == SHAPE_DIAMOND)  M = maskDiamond(pixelUV, coverage);
  else                                   M = coverage;

  if (uEdgeFade > 0.0) {
    vec2 norm = gl_FragCoord.xy / uResolution;
    float edge = min(min(norm.x, norm.y), min(1.0 - norm.x, 1.0 - norm.y));
    float fade = smoothstep(0.0, uEdgeFade, edge);
    M *= fade;
  }

  vec3 color = uColor;
  vec3 srgbColor = mix(
    color * 12.92,
    1.055 * pow(color, vec3(1.0 / 2.4)) - 0.055,
    step(0.0031308, color)
  );

  fragColor = vec4(srgbColor, M);
}
`;

function randomFloat() {
  if (typeof window !== "undefined" && window.crypto?.getRandomValues) {
    const value = new Uint32Array(1);
    window.crypto.getRandomValues(value);
    return value[0] / 0xffffffff;
  }

  return Math.random();
}

export function PixelBlast({
  antialias = true,
  autoPauseOffscreen = true,
  className,
  color = "#B497CF",
  edgeFade = 0.5,
  enableRipples = true,
  liquid = false,
  liquidRadius = 1,
  liquidStrength = 0.1,
  liquidWobbleSpeed = 4.5,
  noiseAmount = 0,
  patternDensity = 1,
  patternScale = 2,
  pixelSize = 3,
  pixelSizeJitter = 0,
  rippleIntensityScale = 1,
  rippleSpeed = 0.3,
  rippleThickness = 0.1,
  speed = 0.5,
  style,
  transparent = true,
  variant = "square",
}: PixelBlastProps) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const speedRef = useRef(speed);
  const threeRef = useRef<PixelBlastRuntime | null>(null);
  const prevConfigRef = useRef<ReinitConfig | null>(null);
  const intersectionVisibleRef = useRef(true);
  const pageVisibleRef = useRef(true);

  useEffect(() => {
    if (!autoPauseOffscreen) {
      intersectionVisibleRef.current = true;
      pageVisibleRef.current = true;
      return undefined;
    }

    const container = containerRef.current;
    if (!container) return undefined;

    const observer = new IntersectionObserver(([entry]) => {
      intersectionVisibleRef.current = entry?.isIntersecting ?? true;
    }, { threshold: 0.01 });

    observer.observe(container);

    const handleVisibilityChange = () => {
      pageVisibleRef.current = !document.hidden;
    };

    handleVisibilityChange();
    document.addEventListener("visibilitychange", handleVisibilityChange);

    return () => {
      observer.disconnect();
      document.removeEventListener("visibilitychange", handleVisibilityChange);
    };
  }, [autoPauseOffscreen]);

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return undefined;

    speedRef.current = speed;

    const needsReinitKeys: Array<keyof ReinitConfig> = ["antialias", "liquid", "noiseAmount"];
    const config: ReinitConfig = { antialias, liquid, noiseAmount };
    let mustReinit = false;

    if (!threeRef.current) {
      mustReinit = true;
    } else if (prevConfigRef.current) {
      for (const key of needsReinitKeys) {
        if (prevConfigRef.current[key] !== config[key]) {
          mustReinit = true;
          break;
        }
      }
    }

    if (mustReinit) {
      if (threeRef.current) {
        const current = threeRef.current;
        current.resizeObserver?.disconnect();
        if (current.raf) cancelAnimationFrame(current.raf);
        current.quad?.geometry.dispose();
        current.material.dispose();
        current.composer?.dispose();
        current.renderer.dispose();
        current.renderer.forceContextLoss();
        if (current.renderer.domElement.parentElement === container) {
          container.removeChild(current.renderer.domElement);
        }
        threeRef.current = null;
      }

      const canvas = document.createElement("canvas");
      const renderer = new THREE.WebGLRenderer({
        alpha: true,
        antialias,
        canvas,
        powerPreference: "high-performance",
      });

      renderer.domElement.style.width = "100%";
      renderer.domElement.style.height = "100%";
      renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
      container.appendChild(renderer.domElement);

      if (transparent) {
        renderer.setClearAlpha(0);
      } else {
        renderer.setClearColor(0x000000, 1);
      }

      const uniforms: PixelBlastUniforms = {
        uClickPos: {
          value: Array.from({ length: MAX_CLICKS }, () => new THREE.Vector2(-1, -1)),
        },
        uClickTimes: { value: new Float32Array(MAX_CLICKS) },
        uColor: { value: new THREE.Color(color) },
        uDensity: { value: patternDensity },
        uEdgeFade: { value: edgeFade },
        uEnableRipples: { value: enableRipples ? 1 : 0 },
        uPixelJitter: { value: pixelSizeJitter },
        uPixelSize: { value: pixelSize * renderer.getPixelRatio() },
        uResolution: { value: new THREE.Vector2(0, 0) },
        uRippleIntensity: { value: rippleIntensityScale },
        uRippleSpeed: { value: rippleSpeed },
        uRippleThickness: { value: rippleThickness },
        uScale: { value: patternScale },
        uShapeType: { value: SHAPE_MAP[variant] ?? 0 },
        uTime: { value: 0 },
      };

      const scene = new THREE.Scene();
      const camera = new THREE.OrthographicCamera(-1, 1, 1, -1, 0, 1);
      const material = new THREE.ShaderMaterial({
        depthTest: false,
        depthWrite: false,
        fragmentShader: FRAGMENT_SRC,
        glslVersion: THREE.GLSL3,
        transparent: true,
        uniforms,
        vertexShader: VERTEX_SRC,
      });
      const quad = new THREE.Mesh(new THREE.PlaneGeometry(2, 2), material);
      scene.add(quad);

      const resize = () => {
        const width = container.clientWidth || 1;
        const height = container.clientHeight || 1;
        renderer.setSize(width, height, false);
        uniforms.uResolution.value.set(renderer.domElement.width, renderer.domElement.height);
        threeRef.current?.composer?.setSize(renderer.domElement.width, renderer.domElement.height);
        uniforms.uPixelSize.value = pixelSize * renderer.getPixelRatio();
      };

      resize();
      const resizeObserver = new ResizeObserver(resize);
      resizeObserver.observe(container);

      const clock = new THREE.Clock();
      const timeOffset = randomFloat() * 1000;

      let composer: EffectComposer | undefined;
      let touch: TouchTexture | undefined;
      let liquidEffect: Effect | undefined;

      if (liquid) {
        touch = createTouchTexture();
        touch.radiusScale = liquidRadius;
        composer = new EffectComposer(renderer);
        composer.addPass(new RenderPass(scene, camera));

        liquidEffect = createLiquidEffect(touch.texture, {
          freq: liquidWobbleSpeed,
          strength: liquidStrength,
        });

        const liquidPass = new EffectPass(camera, liquidEffect);
        liquidPass.renderToScreen = true;
        composer.addPass(liquidPass);
      }

      if (noiseAmount > 0) {
        if (!composer) {
          composer = new EffectComposer(renderer);
          composer.addPass(new RenderPass(scene, camera));
        }

        const noiseEffect = new Effect(
          "NoiseEffect",
          "uniform float uTime; uniform float uAmount; float hash(vec2 p){ return fract(sin(dot(p, vec2(127.1,311.7))) * 43758.5453);} void mainUv(inout vec2 uv){} void mainImage(const in vec4 inputColor,const in vec2 uv,out vec4 outputColor){ float n=hash(floor(uv*vec2(1920.0,1080.0))+floor(uTime*60.0)); float g=(n-0.5)*uAmount; outputColor=inputColor+vec4(vec3(g),0.0);} ",
          {
            uniforms: new Map<string, THREE.Uniform>([
              ["uAmount", new THREE.Uniform(noiseAmount)],
              ["uTime", new THREE.Uniform(0)],
            ]),
          },
        );

        const noisePass = new EffectPass(camera, noiseEffect);
        noisePass.renderToScreen = true;

        composer.passes.forEach((pass) => {
          (pass as { renderToScreen?: boolean }).renderToScreen = false;
        });
        composer.addPass(noisePass);
      }

      composer?.setSize(renderer.domElement.width, renderer.domElement.height);

      const mapToPixels = (event: PointerEvent) => {
        const bounds = renderer.domElement.getBoundingClientRect();
        const scaleX = renderer.domElement.width / bounds.width;
        const scaleY = renderer.domElement.height / bounds.height;
        const fx = (event.clientX - bounds.left) * scaleX;
        const fy = (bounds.height - (event.clientY - bounds.top)) * scaleY;

        return {
          fx,
          fy,
          h: renderer.domElement.height,
          w: renderer.domElement.width,
        };
      };

      const handlePointerDown = (event: PointerEvent) => {
        const { fx, fy } = mapToPixels(event);
        const clickIndex = threeRef.current?.clickIx ?? 0;
        uniforms.uClickPos.value[clickIndex].set(fx, fy);
        uniforms.uClickTimes.value[clickIndex] = uniforms.uTime.value;

        if (threeRef.current) {
          threeRef.current.clickIx = (clickIndex + 1) % MAX_CLICKS;
        }
      };

      const handlePointerMove = (event: PointerEvent) => {
        if (!touch) return;
        const { fx, fy, h, w } = mapToPixels(event);
        touch.addTouch({ x: fx / w, y: fy / h });
      };

      renderer.domElement.addEventListener("pointerdown", handlePointerDown, { passive: true });
      renderer.domElement.addEventListener("pointermove", handlePointerMove, { passive: true });

      const updateEffectTimeUniforms = (nextTime: number) => {
        if (!composer) return;

        composer.passes.forEach((pass) => {
          const effectList = (pass as { effects?: Array<Effect & { uniforms?: Map<string, THREE.Uniform> }> }).effects;
          effectList?.forEach((effect) => {
            const timeUniform = effect.uniforms?.get("uTime");
            if (timeUniform) timeUniform.value = nextTime;
          });
        });
      };

      const animate = () => {
        if (autoPauseOffscreen && (!pageVisibleRef.current || !intersectionVisibleRef.current)) {
          threeRef.current!.raf = requestAnimationFrame(animate);
          return;
        }

        uniforms.uTime.value = timeOffset + clock.getElapsedTime() * speedRef.current;

        const liquidTimeUniform = liquidEffect && (liquidEffect as Effect & { uniforms?: Map<string, THREE.Uniform> }).uniforms?.get("uTime");
        if (liquidTimeUniform) liquidTimeUniform.value = uniforms.uTime.value;

        if (composer) {
          touch?.update();
          updateEffectTimeUniforms(uniforms.uTime.value);
          composer.render();
        } else {
          renderer.render(scene, camera);
        }

        threeRef.current!.raf = requestAnimationFrame(animate);
      };

      const initialRaf = requestAnimationFrame(animate);

      threeRef.current = {
        camera,
        clickIx: 0,
        clock,
        composer,
        liquidEffect,
        material,
        quad,
        raf: initialRaf,
        renderer,
        resizeObserver,
        scene,
        touch,
        uniforms,
      };
      prevConfigRef.current = config;

      return () => {
        renderer.domElement.removeEventListener("pointerdown", handlePointerDown);
        renderer.domElement.removeEventListener("pointermove", handlePointerMove);
        resizeObserver.disconnect();
        if (threeRef.current?.raf) cancelAnimationFrame(threeRef.current.raf);
        quad.geometry.dispose();
        material.dispose();
        composer?.dispose();
        renderer.dispose();
        renderer.forceContextLoss();
        if (renderer.domElement.parentElement === container) {
          container.removeChild(renderer.domElement);
        }
        threeRef.current = null;
      };
    }

    const current = threeRef.current;
    if (!current) return undefined;

    current.uniforms.uShapeType.value = SHAPE_MAP[variant] ?? 0;
    current.uniforms.uPixelSize.value = pixelSize * current.renderer.getPixelRatio();
    current.uniforms.uColor.value.set(color);
    current.uniforms.uScale.value = patternScale;
    current.uniforms.uDensity.value = patternDensity;
    current.uniforms.uPixelJitter.value = pixelSizeJitter;
    current.uniforms.uEnableRipples.value = enableRipples ? 1 : 0;
    current.uniforms.uRippleIntensity.value = rippleIntensityScale;
    current.uniforms.uRippleThickness.value = rippleThickness;
    current.uniforms.uRippleSpeed.value = rippleSpeed;
    current.uniforms.uEdgeFade.value = edgeFade;

    if (transparent) {
      current.renderer.setClearAlpha(0);
    } else {
      current.renderer.setClearColor(0x000000, 1);
    }

    if (current.liquidEffect) {
      const effectUniforms = (current.liquidEffect as Effect & { uniforms?: Map<string, THREE.Uniform> }).uniforms;
      const strengthUniform = effectUniforms?.get("uStrength");
      if (strengthUniform) strengthUniform.value = liquidStrength;
      const freqUniform = effectUniforms?.get("uFreq");
      if (freqUniform) freqUniform.value = liquidWobbleSpeed;
    }

    if (current.touch) {
      current.touch.radiusScale = liquidRadius;
    }

    prevConfigRef.current = config;
    return undefined;
  }, [
    antialias,
    autoPauseOffscreen,
    color,
    edgeFade,
    enableRipples,
    liquid,
    liquidRadius,
    liquidStrength,
    liquidWobbleSpeed,
    noiseAmount,
    patternDensity,
    patternScale,
    pixelSize,
    pixelSizeJitter,
    rippleIntensityScale,
    rippleSpeed,
    rippleThickness,
    speed,
    transparent,
    variant,
  ]);

  return (
    <div
      aria-label="PixelBlast interactive background"
      className={cn("pixel-blast-container", className)}
      ref={containerRef}
      style={style}
    />
  );
}
