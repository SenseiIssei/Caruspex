// The X-ray view in three.js: the glass hull, the parts inside, the control
// units with their bus lines and the packets that run along them. Loaded on
// demand by the X-ray page (three.js is a large module and the start page
// should not wait for it). The page owns the data and the texts; this module
// only draws what layout.ts computed and reports what the mouse points at.
import * as THREE from "three";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import { RoomEnvironment } from "three/examples/jsm/environments/RoomEnvironment.js";
import { RoundedBoxGeometry } from "three/examples/jsm/geometries/RoundedBoxGeometry.js";
import { mergeGeometries } from "three/examples/jsm/utils/BufferGeometryUtils.js";
import { CarZone } from "../api/types";
import type { BusKind, CarLayout, EcuNode, Layer, Material, Part, Tube, Vec3, Wheel } from "./layout";
import { BUSES, LAYERS, lampOutlines } from "./layout";
import { contactAxles, fitShell, looksLikeGlass, looksLikeWheel, orientation, type Bounds, type ShellFit } from "./shell";

export interface XrayPalette {
  light: boolean;
  accent: string;
  sky: string;
  good: string;
  warn: string;
  bad: string;
  info: string;
  text: string;
  muted: string;
  bus: Record<BusKind, string>;
}

export interface XrayLabel {
  id: string;
  anchor: Vec3;
  title: string;
  value?: string;
  tone?: "bad" | "warn" | "good" | "info" | "accent" | "bus";
  // The bus whose colour a "bus" label takes.
  bus?: BusKind;
}

// The section plane: across the car (x, cut from the front), along it (z,
// cut from the driver's side) or level (y, cut from above), and where it
// stands from 0 to 1 over the car.
export type SectionAxis = "x" | "y" | "z";
export interface Section {
  axis: SectionAxis;
  t: number;
}

// What the frame counter measured over the last half second.
export interface FrameStats {
  fps: number;
  frameMs: number;
  renderMs: number;
  calls: number;
  triangles: number;
}

export interface Highlight {
  zones: CarZone[];
  // The zone or unit the camera looks at; everything else steps back.
  focusZone: CarZone | null;
  focusEcu: string | null;
  selectedEcu: string | null;
}

export interface SceneEvents {
  onHover?: (id: string | null) => void;
  onPick?: (id: string | null) => void;
  // The section plane was dragged in the view.
  onSection?: (t: number) => void;
  onStats?: (stats: FrameStats) => void;
}

const GLASS_VERTEX = /* glsl */ `
#include <clipping_planes_pars_vertex>
attribute float aTop;
varying vec3 vWorld;
varying vec3 vNormalW;
varying float vTop;
void main() {
  vec4 world = modelMatrix * vec4(position, 1.0);
  vWorld = world.xyz;
  vNormalW = normalize(mat3(modelMatrix) * normal);
  vTop = aTop;
  vec4 mvPosition = viewMatrix * world;
  gl_Position = projectionMatrix * mvPosition;
  #include <clipping_planes_vertex>
}
`;

const GLASS_FRAGMENT = /* glsl */ `
uniform vec3 uTint;
uniform vec3 uRim;
uniform vec3 uWindow;
uniform vec3 uSky;
uniform vec3 uGround;
uniform float uOpacity;
uniform float uWindowAlpha;
uniform float uFade;
uniform float uBelt;
uniform float uBeltRear;
uniform float uRear;
uniform float uLength;
uniform float uCowl;
uniform float uRoofFront;
uniform float uRoofRear;
uniform float uTail;
uniform vec4 uDlo;
uniform vec4 uPillars;
uniform vec4 uPillarW;
uniform float uKink;
uniform float uGraphic;
uniform float uWindowAll;
varying vec3 vWorld;
varying vec3 vNormalW;
varying float vTop;
#include <clipping_planes_pars_fragment>

float band(float a, float b, float soft, float x) {
  return smoothstep(a, a + soft, x) * (1.0 - smoothstep(b - soft, b, x));
}

void main() {
  #include <clipping_planes_fragment>
  vec3 n = normalize(vNormalW);
  if (!gl_FrontFacing) n = -n;
  vec3 v = normalize(cameraPosition - vWorld);
  float ndv = clamp(abs(dot(n, v)), 0.0, 1.0);
  float fresnel = pow(1.0 - ndv, 2.4);
  float u = (vWorld.x - uRear) / uLength;
  float y = vWorld.y;

  // Glass areas: the side window opening (its front edge along the
  // A-pillar, its rear edge along the last pillar with the kink at the
  // bottom), split by the pillars between the doors; windscreen; rear window.
  float side = smoothstep(0.42, 0.62, abs(vNormalW.z));
  float beltY = mix(uBeltRear, uBelt, smoothstep(uDlo.z, uCowl, u));
  float k = clamp((y - beltY) / max(0.05, vTop - beltY), 0.0, 1.0);
  float frontEdge = mix(uDlo.x, uDlo.y, k);
  float rearEdge = mix(uDlo.z, uDlo.w, k) + uKink * (1.0 - smoothstep(0.0, 0.4, k));
  float sideWin = side * band(beltY + 0.015, vTop - 0.045, 0.012, y) * band(rearEdge, frontEdge, 0.005, u);
  for (int i = 0; i < 4; i++) {
    float w = uPillarW[i];
    if (w > 0.0) sideWin *= smoothstep(w * 0.5, w * 0.5 + 0.003, abs(u - uPillars[i]));
  }
  float frontal = 1.0 - smoothstep(0.32, 0.55, abs(vNormalW.z));
  float notRoof = 1.0 - smoothstep(0.96, 0.995, vNormalW.y);
  float shield = frontal * band(uRoofFront + 0.004, uCowl, 0.012, u) * smoothstep(uBelt, uBelt + 0.03, y) * notRoof;
  float rearWin = frontal * band(uTail, uRoofRear - 0.004, 0.01, u) * smoothstep(uBeltRear, uBeltRear + 0.03, y) * notRoof;
  float win = clamp((sideWin + shield + rearWin) * uGraphic + uWindowAll, 0.0, 1.0);

  vec3 r = reflect(-v, n);
  float skyAmount = smoothstep(-0.25, 0.55, r.y);
  vec3 env = mix(uGround, uSky, skyAmount);
  // A soft studio light strip along the upper body.
  float strip = smoothstep(0.55, 0.9, r.y) * (1.0 - smoothstep(0.9, 0.99, r.y));

  vec3 base = mix(uTint, uWindow, win);
  vec3 col = mix(base, env, 0.35 + fresnel * 0.45);
  col += uRim * (fresnel * 0.9 + strip * 0.18);
  float alpha = mix(uOpacity, uWindowAlpha, win) + fresnel * (0.5 - 0.2 * win) + strip * 0.06;
  gl_FragColor = vec4(col, clamp(alpha, 0.0, 0.92) * uFade);
  #include <colorspace_fragment>
}
`;

interface Packet {
  curve: THREE.CatmullRomCurve3;
  length: number;
  t: number;
  mesh: THREE.Mesh;
  glow: THREE.Sprite;
  target: string;
}

interface Tween {
  fromPos: THREE.Vector3;
  toPos: THREE.Vector3;
  fromTarget: THREE.Vector3;
  toTarget: THREE.Vector3;
  start: number;
  duration: number;
}

const v3 = (p: Vec3) => new THREE.Vector3(p[0], p[1], p[2]);
const ease = (t: number) => (t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2);

// Polyline with rounded corners, dense enough for a crisp tube.
function roundedPath(points: Vec3[], radius: number): THREE.Vector3[] {
  const pts = points.map(v3);
  if (pts.length < 3) return pts;
  const out: THREE.Vector3[] = [pts[0]!];
  for (let i = 1; i < pts.length - 1; i++) {
    const prev = pts[i - 1]!;
    const cur = pts[i]!;
    const next = pts[i + 1]!;
    const a = prev.clone().sub(cur);
    const b = next.clone().sub(cur);
    const r = Math.min(radius, a.length() / 2, b.length() / 2);
    if (r < 1e-3) {
      out.push(cur);
      continue;
    }
    const p0 = cur.clone().add(a.normalize().multiplyScalar(r));
    const p1 = cur.clone().add(b.normalize().multiplyScalar(r));
    for (let k = 0; k <= 6; k++) {
      const t = k / 6;
      // Quadratic Bezier through the corner.
      const q = p0.clone().multiplyScalar((1 - t) * (1 - t)).add(cur.clone().multiplyScalar(2 * (1 - t) * t)).add(p1.clone().multiplyScalar(t * t));
      out.push(q);
    }
  }
  out.push(pts[pts.length - 1]!);
  return out;
}

function pathCurve(points: Vec3[], radius = 0.06): THREE.CatmullRomCurve3 {
  return new THREE.CatmullRomCurve3(roundedPath(points, radius), false, "centripetal", 0.0);
}

function glowTexture(): THREE.Texture {
  const size = 64;
  const canvas = document.createElement("canvas");
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext("2d")!;
  const g = ctx.createRadialGradient(size / 2, size / 2, 0, size / 2, size / 2, size / 2);
  g.addColorStop(0, "rgba(255,255,255,1)");
  g.addColorStop(0.25, "rgba(255,255,255,0.55)");
  g.addColorStop(1, "rgba(255,255,255,0)");
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, size, size);
  const tex = new THREE.CanvasTexture(canvas);
  tex.colorSpace = THREE.SRGBColorSpace;
  return tex;
}

function shadowTexture(light: boolean): THREE.Texture {
  const w = 256;
  const h = 128;
  const canvas = document.createElement("canvas");
  canvas.width = w;
  canvas.height = h;
  const ctx = canvas.getContext("2d")!;
  ctx.translate(w / 2, h / 2);
  ctx.scale(1, h / w);
  const g = ctx.createRadialGradient(0, 0, 0, 0, 0, w / 2);
  const c = light ? "20,28,48" : "0,0,0";
  g.addColorStop(0, `rgba(${c},${light ? 0.32 : 0.6})`);
  g.addColorStop(0.55, `rgba(${c},${light ? 0.14 : 0.3})`);
  g.addColorStop(1, `rgba(${c},0)`);
  ctx.fillStyle = g;
  ctx.fillRect(-w / 2, -w / 2, w, w);
  const tex = new THREE.CanvasTexture(canvas);
  tex.colorSpace = THREE.SRGBColorSpace;
  return tex;
}

function floorTexture(light: boolean, accent: string): THREE.Texture {
  const size = 512;
  const canvas = document.createElement("canvas");
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext("2d")!;
  const color = new THREE.Color(accent);
  const rgb = `${Math.round(color.r * 255)},${Math.round(color.g * 255)},${Math.round(color.b * 255)}`;
  // Concentric rings that fade out: a turntable under the car.
  for (let i = 1; i <= 7; i++) {
    const r = (i / 7) * (size / 2 - 4);
    ctx.beginPath();
    ctx.arc(size / 2, size / 2, r, 0, Math.PI * 2);
    ctx.strokeStyle = `rgba(${rgb},${(light ? 0.22 : 0.16) * (1 - i / 8)})`;
    ctx.lineWidth = i === 7 ? 1.5 : 1;
    ctx.stroke();
  }
  const tex = new THREE.CanvasTexture(canvas);
  tex.colorSpace = THREE.SRGBColorSpace;
  return tex;
}

// The outline strokes as pairs of points (x, y, z, x, y, z, ...).
export function outlineSegments(layout: CarLayout): number[] {
  const hull = layout.hull;
  const p = hull.profile;
  const segments: number[] = [];
  const at = (u: number, y: number, side: number): Vec3 => [hull.xAt(u), y, side * (hull.halfWidth(u, y) + 0.003)];
  const line = (pts: Vec3[]) => {
    for (let i = 0; i + 1 < pts.length; i++) segments.push(...pts[i]!, ...pts[i + 1]!);
  };
  const sample = (from: number, to: number, f: (t: number) => Vec3, n = 40) => {
    const out: Vec3[] = [];
    for (let i = 0; i <= n; i++) out.push(f(from + ((to - from) * i) / n));
    return out;
  };
  const [frontBottom, frontTop, rearBottom, rearTop] = p.dlo;
  const belt = (u: number) => hull.beltAt(u) + 0.015;
  const roof = (u: number) => hull.top(u) - 0.045;
  const kinked = (k: number) => p.kink * (1 - Math.min(1, k / 0.4));
  for (const side of [1, -1]) {
    // Window opening: window line, A-pillar, roof edge, last pillar.
    if (rearBottom < frontBottom) {
      line(sample(rearBottom + p.kink, frontBottom, (u) => at(u, belt(u), side)));
      line(sample(0, 1, (k) => {
        const u = frontBottom + (frontTop - frontBottom) * k;
        return at(u, Math.min(roof(u), belt(frontBottom) + (roof(frontTop) - belt(frontBottom)) * k), side);
      }, 16));
      if (rearTop < frontTop) line(sample(rearTop, frontTop, (u) => at(u, roof(u), side)));
      line(sample(0, 1, (k) => {
        const u = rearBottom + (rearTop - rearBottom) * k + kinked(k);
        return at(u, Math.min(roof(u), belt(rearBottom) + (roof(rearTop) - belt(rearBottom)) * k), side);
      }, 16));
      for (const pillar of p.pillars) {
        if (roof(pillar) > belt(pillar)) line(sample(belt(pillar), roof(pillar), (y) => at(pillar, y, side), 10));
      }
    }
    // Door seams below the window line.
    for (const seam of p.seams) {
      const u = hull.station(seam.x);
      if (seam.top > seam.bottom + 0.05) line(sample(seam.top, seam.bottom, (y) => at(u, y, side), 12));
    }
    // Wheel arches.
    for (const w of layout.wheels.filter((w) => Math.sign(w.center[2]) === side)) {
      const r = w.radius + 0.05;
      const pts: Vec3[] = [];
      for (let i = 0; i <= 24; i++) {
        const a = Math.PI * 0.08 + (Math.PI * 0.84 * i) / 24;
        const x = w.center[0] + Math.cos(a) * r;
        const y = w.center[1] + Math.sin(a) * r;
        const u = hull.station(x);
        const half = hull.halfWidth(u, Math.max(y, hull.bottom(u) + 0.01));
        pts.push([x, y, side * Math.max(half, Math.abs(w.center[2]) + w.width / 2) + side * 0.004]);
      }
      line(pts);
    }
  }
  // Base of the windscreen and of the rear window, across the car.
  for (const u of [p.cowl, p.tail + 0.004]) {
    const y = hull.beltAt(u) + 0.02;
    const half = hull.halfWidth(u, y);
    if (half <= 0) continue;
    const pts: Vec3[] = [];
    for (let i = 0; i <= 20; i++) pts.push([hull.xAt(u), y + 0.004, -half + (2 * half * i) / 20]);
    line(pts);
  }
  return segments;
}

export class XrayScene {
  private renderer: THREE.WebGLRenderer;
  private scene = new THREE.Scene();
  private camera: THREE.PerspectiveCamera;
  private controls: OrbitControls;
  private container: HTMLElement;
  private overlay: HTMLDivElement;
  private palette: XrayPalette;
  private reducedMotion: boolean;
  private events: SceneEvents;

  private layerGroups = new Map<Layer, THREE.Group>();
  private wheelGroup = new THREE.Group();
  private floorGroup = new THREE.Group();
  private networkGroup = new THREE.Group();
  private packetGroup = new THREE.Group();
  private glass: THREE.ShaderMaterial[] = [];
  private partMeshes = new Map<string, THREE.Mesh>();
  private partInfo = new Map<string, { zones: CarZone[]; baseOpacity: number; color: THREE.Color }>();
  private ecuMeshes = new Map<string, THREE.Mesh>();
  private ecuNodes = new Map<string, EcuNode>();
  private routes = new Map<string, { curve: THREE.CatmullRomCurve3; bus: BusKind }>();
  private diagRoute: { curve: THREE.CatmullRomCurve3; bus: BusKind } | null = null;
  private packets: Packet[] = [];
  private flashes = new Map<string, number>();
  // The labels with their measured size (measured once per text, so placing
  // them every frame does not make the page lay itself out again).
  private labels = new Map<string, { el: HTMLDivElement; data: XrayLabel; w?: number; h?: number }>();
  private highlight: Highlight = { zones: [], focusZone: null, focusEcu: null, selectedEcu: null };
  private visible: Record<Layer, boolean> = { body: true, drivetrain: true, cooling: true, energy: true, electrics: true, network: true };
  private layout: CarLayout | null = null;
  private tween: Tween | null = null;
  private raf = 0;
  private active = true;
  private dirty = true;
  private last = 0;
  private hovered: string | null = null;
  private pointer: { x: number; y: number } | null = null;
  private downAt: { x: number; y: number } | null = null;
  private raycaster = new THREE.Raycaster();
  private glow: THREE.Texture;
  private resize: ResizeObserver;
  private disposed = false;
  private home = { position: new THREE.Vector3(), target: new THREE.Vector3() };
  private shell: { outer: THREE.Group; inner: THREE.Object3D; turned: boolean; fit: ShellFit | null } | null = null;
  // The bodywork of a motorcycle in glass (not picked, not highlighted).
  private shells: THREE.Mesh[] = [];
  // The exploded view: every part of the drivetrain with its place and the
  // way it moves out; the share it has moved and where it is heading.
  private explodeMeshes: { mesh: THREE.Object3D; home: THREE.Vector3; offset: THREE.Vector3 }[] = [];
  private explodeTarget = 0;
  private explodeT = 0;
  // The section plane: one plane shared by every material of the car (an
  // empty list when off), its frame in the view and the drag on it.
  private clip: THREE.Plane[] = [];
  private clipPlane = new THREE.Plane(new THREE.Vector3(-1, 0, 0), 0);
  private section: Section | null = null;
  private sectionGroup = new THREE.Group();
  private sectionHandle: THREE.Mesh | null = null;
  private dragging = false;
  // The frame counter of the demo (statsOn: draw every frame and measure).
  private statsOn = false;
  private statsFrames: number[] = [];
  private statsRender: number[] = [];
  private statsAt = 0;

  constructor(container: HTMLElement, palette: XrayPalette, reducedMotion: boolean, events: SceneEvents = {}) {
    this.container = container;
    this.palette = palette;
    this.reducedMotion = reducedMotion;
    this.events = events;
    this.renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true, powerPreference: "high-performance" });
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
    this.renderer.outputColorSpace = THREE.SRGBColorSpace;
    this.renderer.toneMapping = THREE.ACESFilmicToneMapping;
    this.renderer.toneMappingExposure = 1.05;
    this.renderer.setClearColor(0x000000, 0);
    this.renderer.localClippingEnabled = true;
    const canvas = this.renderer.domElement;
    canvas.className = "xray-canvas";
    container.appendChild(canvas);

    this.overlay = document.createElement("div");
    this.overlay.className = "xray-labels";
    container.appendChild(this.overlay);

    this.camera = new THREE.PerspectiveCamera(34, 1, 0.05, 80);
    this.controls = new OrbitControls(this.camera, canvas);
    this.controls.enableDamping = !reducedMotion;
    this.controls.dampingFactor = 0.08;
    this.controls.screenSpacePanning = true;
    this.controls.maxPolarAngle = Math.PI * 0.495;
    this.controls.addEventListener("change", () => (this.dirty = true));

    const pmrem = new THREE.PMREMGenerator(this.renderer);
    this.scene.environment = pmrem.fromScene(new RoomEnvironment(), 0.04).texture;
    pmrem.dispose();
    this.scene.add(new THREE.HemisphereLight(0xffffff, 0x404858, 0.9));
    const sun = new THREE.DirectionalLight(0xffffff, 1.4);
    sun.position.set(3, 6, 4);
    this.scene.add(sun);

    for (const layer of LAYERS) {
      const g = new THREE.Group();
      g.name = layer;
      this.layerGroups.set(layer, g);
      this.scene.add(g);
    }
    this.layerGroups.get("network")!.add(this.networkGroup);
    this.layerGroups.get("network")!.add(this.packetGroup);
    this.scene.add(this.wheelGroup);
    this.scene.add(this.floorGroup);
    this.sectionGroup.visible = false;
    this.scene.add(this.sectionGroup);
    this.glow = glowTexture();

    // Before the orbit controls see the press: a press on the section plane
    // drags the plane instead of turning the view.
    container.addEventListener("pointerdown", this.onSectionDown, true);
    canvas.addEventListener("pointermove", this.onPointerMove);
    canvas.addEventListener("pointerleave", this.onPointerLeave);
    canvas.addEventListener("pointerdown", this.onPointerDown);
    canvas.addEventListener("pointerup", this.onPointerUp);
    container.addEventListener("keydown", this.onKeyDown);

    this.resize = new ResizeObserver(() => this.fit());
    this.resize.observe(container);
    this.fit();
    this.raf = requestAnimationFrame(this.loop);
  }

  // --- Building ---------------------------------------------------------------------

  // The car itself: hull, wheels, parts and tubes. Called when the car or
  // the theme changes; the network has its own call.
  setCar(layout: CarLayout, keepView = false): void {
    this.layout = layout;
    for (const layer of LAYERS) {
      const g = this.layerGroups.get(layer)!;
      for (const child of [...g.children]) {
        if (child === this.networkGroup || child === this.packetGroup) continue;
        g.remove(child);
        if (child !== this.shell?.outer) disposeTree(child);
      }
    }
    for (const g of [this.wheelGroup, this.floorGroup]) {
      for (const child of [...g.children]) {
        g.remove(child);
        disposeTree(child);
      }
    }
    this.partMeshes.clear();
    this.partInfo.clear();
    this.glass = [];
    this.shells = [];
    this.explodeMeshes = [];

    // A motorcycle has no closed body: its bodywork comes as glass parts.
    if (layout.vehicle === "car") this.buildHull(layout);
    this.buildWheels(layout);
    this.wheelGroup.visible = true;
    this.buildFloor(layout);
    for (const part of layout.parts) this.addPart(part);
    for (const tube of layout.tubes) this.addTube(tube);
    this.setNetwork(layout);
    if (this.shell) {
      this.shell.outer.parent?.remove(this.shell.outer);
      this.placeShell();
    }
    this.applyHighlight();
    this.applyLayers();
    this.applyExplode();
    this.placeSection();
    this.applyClipping();

    const L = layout.hull.length;
    const H = layout.hull.height;
    this.controls.minDistance = Math.max(0.8, L * 0.2);
    this.controls.maxDistance = L * 3.2;
    // Three quarters from the front left, far enough for the whole car, also
    // when the stage is narrow (the near front corner comes out large).
    const vfov = THREE.MathUtils.degToRad(this.camera.fov);
    const hfov = 2 * Math.atan(Math.tan(vfov / 2) * Math.max(0.5, this.camera.aspect));
    const distance = ((L * 0.5) / Math.tan(Math.min(vfov, hfov) / 2)) * 1.12;
    this.home.target.set(0, H * 0.38, 0);
    this.home.position.copy(new THREE.Vector3(0.62, 0.55, 0.62).normalize().multiplyScalar(distance).add(this.home.target));
    if (!keepView) {
      this.camera.position.copy(this.home.position);
      this.controls.target.copy(this.home.target);
      this.controls.update();
    }
    this.dirty = true;
  }

  private buildHull(layout: CarLayout): void {
    const hull = layout.hull;
    const p = hull.profile;
    const S = 160;
    const R = 96;
    const positions = new Float32Array((S + 1) * R * 3);
    const tops = new Float32Array((S + 1) * R);
    for (let i = 0; i <= S; i++) {
      const t = i / S;
      const u = 0.5 - 0.5 * Math.cos(Math.PI * t);
      const top = hull.top(u);
      for (let k = 0; k < R; k++) {
        const theta = -Math.PI / 2 + (2 * Math.PI * k) / R;
        const q = hull.section(u, theta);
        const at = (i * R + k) * 3;
        positions[at] = q[0];
        positions[at + 1] = q[1];
        positions[at + 2] = q[2];
        tops[i * R + k] = top;
      }
    }
    const index: number[] = [];
    for (let i = 0; i < S; i++) {
      for (let k = 0; k < R; k++) {
        const a = i * R + k;
        const b = i * R + ((k + 1) % R);
        const c = (i + 1) * R + k;
        const d = (i + 1) * R + ((k + 1) % R);
        index.push(a, c, b, b, c, d);
      }
    }
    const geo = new THREE.BufferGeometry();
    geo.setAttribute("position", new THREE.BufferAttribute(positions, 3));
    geo.setAttribute("aTop", new THREE.BufferAttribute(tops, 1));
    geo.setIndex(index);
    geo.computeVertexNormals();

    const pal = this.palette;
    const tint = pal.light ? new THREE.Color("#5f7196") : new THREE.Color(pal.accent).lerp(new THREE.Color(pal.sky), 0.45).multiplyScalar(0.55);
    const rim = pal.light ? new THREE.Color("#2b3b63") : new THREE.Color(pal.sky).lerp(new THREE.Color("#ffffff"), 0.25);
    const windowTint = pal.light ? new THREE.Color("#22304f") : new THREE.Color("#03060c");
    const sky = pal.light ? new THREE.Color("#f2f5fb") : new THREE.Color("#2a3552");
    const ground = pal.light ? new THREE.Color("#8a93a8") : new THREE.Color("#06080d");
    const pillars = new THREE.Vector4(...([0, 1, 2, 3].map((i) => p.pillars[i] ?? -1) as [number, number, number, number]));
    const pillarWidths = new THREE.Vector4(...([0, 1, 2, 3].map((i) => p.pillarWidths[i] ?? 0) as [number, number, number, number]));
    const dlo = new THREE.Vector4(...p.dlo);
    const make = (side: THREE.Side, opacity: number, windowAlpha: number, order: number) => {
      const m = new THREE.ShaderMaterial({
        vertexShader: GLASS_VERTEX,
        fragmentShader: GLASS_FRAGMENT,
        transparent: true,
        depthWrite: false,
        clipping: true,
        side,
        uniforms: {
          uTint: { value: tint },
          uRim: { value: rim },
          uWindow: { value: windowTint },
          uSky: { value: sky },
          uGround: { value: ground },
          uOpacity: { value: opacity },
          uWindowAlpha: { value: windowAlpha },
          uFade: { value: 1 },
          uBelt: { value: hull.belt },
          uBeltRear: { value: p.beltRear * hull.height },
          uRear: { value: hull.rear },
          uLength: { value: hull.length },
          uCowl: { value: p.cowl },
          uRoofFront: { value: p.roofFront },
          uRoofRear: { value: p.roofRear },
          uTail: { value: p.tail },
          uDlo: { value: dlo },
          uPillars: { value: pillars },
          uPillarW: { value: pillarWidths },
          uKink: { value: p.kink },
          uGraphic: { value: 1 },
          uWindowAll: { value: 0 },
        },
      });
      const mesh = new THREE.Mesh(geo, m);
      mesh.renderOrder = order;
      mesh.name = "hull";
      this.layerGroups.get("body")!.add(mesh);
      this.glass.push(m);
    };
    make(THREE.BackSide, pal.light ? 0.05 : 0.035, pal.light ? 0.16 : 0.2, 10);
    make(THREE.FrontSide, pal.light ? 0.07 : 0.05, pal.light ? 0.34 : 0.4, 11);
    this.buildOutline(layout, rim);
    this.buildLamps(layout, rim);
  }

  // Headlights, tail lights and the grille opening as lines on the glass,
  // so the front and the rear read as such from any side: lamps in their
  // light colours, the openings in the colour of the outline.
  private buildLamps(layout: CarLayout, rim: THREE.Color): void {
    const lamps = lampOutlines(layout.hull);
    const light = this.palette.light;
    const group = new THREE.Group();
    group.name = "lamps";
    const add = (loops: Vec3[][], color: THREE.Color, opacity: number) => {
      const segments: number[] = [];
      for (const l of loops) for (let i = 0; i + 1 < l.length; i++) segments.push(...l[i]!, ...l[i + 1]!);
      if (!segments.length) return;
      const geo = new THREE.BufferGeometry();
      geo.setAttribute("position", new THREE.Float32BufferAttribute(segments, 3));
      const mat = new THREE.LineBasicMaterial({ color, transparent: true, opacity, depthWrite: false, toneMapped: false });
      const lines = new THREE.LineSegments(geo, mat);
      lines.renderOrder = 12;
      group.add(lines);
    };
    add(lamps.head, new THREE.Color(light ? "#33476f" : "#f2f7ff"), light ? 0.75 : 0.8);
    add(lamps.tail, new THREE.Color(light ? "#c0362c" : "#ff5f57"), light ? 0.8 : 0.85);
    add(lamps.grille, rim, light ? 0.5 : 0.45);
    this.layerGroups.get("body")!.add(group);
  }

  // Thin lines along the window opening, the pillars, the door seams and
  // the wheel arches: the few strokes that make a glass blob read as a car.
  private buildOutline(layout: CarLayout, color: THREE.Color): void {
    const segments = outlineSegments(layout);
    const geo = new THREE.BufferGeometry();
    geo.setAttribute("position", new THREE.Float32BufferAttribute(segments, 3));
    const mat = new THREE.LineBasicMaterial({ color, transparent: true, opacity: this.palette.light ? 0.45 : 0.4, depthWrite: false, toneMapped: false });
    const lines = new THREE.LineSegments(geo, mat);
    lines.renderOrder = 12;
    lines.name = "outline";
    this.layerGroups.get("body")!.add(lines);
  }

  // The wheels: per wheel one mesh for the tyre, one for rim, spokes and hub
  // merged, and the brake discs and calipers, which glow on a fault.
  private buildWheels(layout: CarLayout): void {
    const pal = this.palette;
    const tyreMat = new THREE.MeshStandardMaterial({ color: pal.light ? 0x2a2d34 : 0x15171c, roughness: 0.85, metalness: 0.05 });
    const rimMat = new THREE.MeshStandardMaterial({ color: pal.light ? 0x9aa3b2 : 0xb9c2cf, roughness: 0.28, metalness: 0.9 });
    const discMat = new THREE.MeshStandardMaterial({ color: 0x5b6270, roughness: 0.5, metalness: 0.8 });
    const caliperMat = new THREE.MeshStandardMaterial({ color: new THREE.Color(pal.accent), roughness: 0.4, metalness: 0.3 });
    for (const w of layout.wheels) {
      const g = w.moto ? this.motoWheel(w, tyreMat, rimMat, discMat, caliperMat) : this.carWheel(w, tyreMat, rimMat, discMat, caliperMat);
      g.position.set(w.center[0], w.center[1], w.center[2]);
      g.userData.zones = [CarZone.Wheels];
      this.wheelGroup.add(g);
    }
  }

  private carWheel(w: Wheel, tyreMat: THREE.Material, rimMat: THREE.Material, discMat: THREE.MeshStandardMaterial, caliperMat: THREE.MeshStandardMaterial): THREE.Group {
    const g = new THREE.Group();
    const R = w.radius;
    const half = w.width / 2;
    const inner = R * 0.66;
    // Tyre: a rounded profile turned round the axle.
    const profile: THREE.Vector2[] = [];
    const shoulder = Math.min(0.045, half * 0.4);
    profile.push(new THREE.Vector2(inner, -half * 0.92));
    for (let k = 0; k <= 6; k++) {
      const a = -Math.PI / 2 + (k / 6) * (Math.PI / 2);
      profile.push(new THREE.Vector2(R - shoulder + Math.cos(a) * shoulder, -half + shoulder + Math.sin(a) * shoulder));
    }
    for (let k = 0; k <= 6; k++) {
      const a = (k / 6) * (Math.PI / 2);
      profile.push(new THREE.Vector2(R - shoulder + Math.cos(a) * shoulder, half - shoulder + Math.sin(a) * shoulder));
    }
    profile.push(new THREE.Vector2(inner, half * 0.92));
    const tyreGeo = new THREE.LatheGeometry(profile, 48);
    tyreGeo.rotateX(Math.PI / 2);
    g.add(new THREE.Mesh(tyreGeo, tyreMat));
    const out = Math.sign(w.center[2]) || 1;
    // Rim, five spokes on the outer face and the hub as one mesh.
    const pieces: THREE.BufferGeometry[] = [];
    pieces.push(new THREE.CylinderGeometry(inner, inner, w.width * 0.8, 40).rotateX(Math.PI / 2));
    for (let s = 0; s < 5; s++) {
      const a = (s / 5) * Math.PI * 2;
      pieces.push(new THREE.BoxGeometry(inner * 0.9, inner * 0.16, 0.025).rotateZ(a).translate(Math.cos(a) * inner * 0.48, Math.sin(a) * inner * 0.48, out * (w.width * 0.42)));
    }
    pieces.push(new THREE.CylinderGeometry(inner * 0.22, inner * 0.22, 0.03, 20).rotateX(Math.PI / 2).translate(0, 0, out * w.width * 0.43));
    g.add(new THREE.Mesh(merged(pieces), rimMat));
    const disc = new THREE.Mesh(new THREE.CylinderGeometry(inner * 0.82, inner * 0.82, 0.028, 36).rotateX(Math.PI / 2), discMat.clone());
    disc.position.z = -out * w.width * 0.1;
    disc.userData.glow = true;
    g.add(disc);
    const caliper = new THREE.Mesh(new RoundedBoxGeometry(0.07, inner * 0.55, 0.06, 2, 0.015), caliperMat.clone());
    caliper.position.set(w.front ? -inner * 0.62 : inner * 0.62, inner * 0.25, -out * w.width * 0.05);
    caliper.userData.glow = true;
    g.add(caliper);
    return g;
  }

  // A motorcycle wheel: a round tyre on a narrow rim, cast spokes or wire
  // spokes, the discs on one or both sides with their calipers.
  private motoWheel(w: Wheel, tyreMat: THREE.Material, rimMat: THREE.Material, discMat: THREE.MeshStandardMaterial, caliperMat: THREE.MeshStandardMaterial): THREE.Group {
    const g = new THREE.Group();
    const R = w.radius;
    const half = w.width / 2;
    const rim = R - w.width * 0.78;
    const profile: THREE.Vector2[] = [];
    for (let k = 0; k <= 16; k++) {
      const a = -Math.PI * 0.42 + (k / 16) * Math.PI * 0.84;
      // A round crown: wide at the sides, round over the tread.
      profile.push(new THREE.Vector2(rim + (R - rim) * Math.cos(a) ** 0.6, half * Math.sin(a) * 1.05));
    }
    const tyreGeo = new THREE.LatheGeometry(profile, 48);
    tyreGeo.rotateX(Math.PI / 2);
    g.add(new THREE.Mesh(tyreGeo, tyreMat));
    const pieces: THREE.BufferGeometry[] = [];
    pieces.push(new THREE.CylinderGeometry(rim, rim, w.width * 0.7, 40, 1, true).rotateX(Math.PI / 2));
    pieces.push(new THREE.CylinderGeometry(0.045, 0.045, w.width * 0.9, 16).rotateX(Math.PI / 2));
    const spokes = w.moto!.spokes === "wire" ? 28 : 6;
    for (let s = 0; s < spokes; s++) {
      const a = (s / spokes) * Math.PI * 2;
      const thick = w.moto!.spokes === "wire" ? 0.004 : 0.02;
      const side = w.moto!.spokes === "wire" ? (s % 2 ? 1 : -1) * half * 0.35 : 0;
      pieces.push(new THREE.BoxGeometry(rim - 0.04, thick, thick * 1.4).translate((rim - 0.04) / 2 + 0.03, 0, side).rotateZ(a));
    }
    g.add(new THREE.Mesh(merged(pieces), rimMat));
    for (const d of w.moto!.discs) {
      const z = d.side * (half * 0.75 + 0.03);
      const disc = new THREE.Mesh(new THREE.CylinderGeometry(d.radius, d.radius, 0.006, 40).rotateX(Math.PI / 2), discMat.clone());
      disc.position.z = z;
      disc.userData.glow = true;
      g.add(disc);
      const caliper = new THREE.Mesh(new RoundedBoxGeometry(0.06, 0.1, 0.045, 2, 0.012), caliperMat.clone());
      const a = w.front ? Math.PI * 0.8 : Math.PI * 1.15;
      caliper.position.set(Math.cos(a) * d.radius * 0.9, Math.sin(a) * d.radius * 0.9, z + d.side * 0.012);
      caliper.rotation.z = a - Math.PI / 2;
      caliper.userData.glow = true;
      g.add(caliper);
    }
    return g;
  }

  private buildFloor(layout: CarLayout): void {
    const L = layout.hull.length;
    const W = layout.hull.width;
    const shadow = new THREE.Mesh(
      new THREE.PlaneGeometry(L * 1.35, W * 1.9),
      new THREE.MeshBasicMaterial({ map: shadowTexture(this.palette.light), transparent: true, depthWrite: false, toneMapped: false }),
    );
    shadow.rotation.x = -Math.PI / 2;
    shadow.position.y = 0.002;
    shadow.renderOrder = 1;
    this.floorGroup.add(shadow);
    const ring = new THREE.Mesh(
      new THREE.PlaneGeometry(L * 2.3, L * 2.3),
      new THREE.MeshBasicMaterial({ map: floorTexture(this.palette.light, this.palette.accent), transparent: true, depthWrite: false, toneMapped: false }),
    );
    ring.rotation.x = -Math.PI / 2;
    ring.position.y = 0.001;
    ring.renderOrder = 0;
    this.floorGroup.add(ring);
  }

  private materialColor(m: Material): { color: THREE.Color; metal: number; rough: number; opacity: number } {
    const p = this.palette;
    const c = (hex: string) => new THREE.Color(hex);
    switch (m) {
      case "engine": return { color: c(p.light ? "#5b6476" : "#7d8799"), metal: 0.65, rough: 0.42, opacity: 1 };
      case "metal": return { color: c(p.light ? "#7c8698" : "#a3acbb"), metal: 0.8, rough: 0.35, opacity: 1 };
      case "gearbox": return { color: c(p.light ? "#6a5fa8" : "#9a8cff"), metal: 0.55, rough: 0.4, opacity: 1 };
      case "exhaust": return { color: c(p.light ? "#9a6a3e" : "#c99766"), metal: 0.7, rough: 0.45, opacity: 1 };
      case "cooling": return { color: c(p.info), metal: 0.3, rough: 0.45, opacity: 1 };
      case "fuel": return { color: c(p.warn), metal: 0.2, rough: 0.5, opacity: 1 };
      case "hv": return { color: c(p.light ? "#e0660f" : "#ff8a3d"), metal: 0.2, rough: 0.5, opacity: 1 };
      case "battery": return { color: c(p.light ? "#1f9e8c" : "#34d1b8"), metal: 0.3, rough: 0.45, opacity: 1 };
      case "electric": return { color: c(p.light ? "#3b6fd8" : "#6fa8ff"), metal: 0.3, rough: 0.5, opacity: 1 };
      case "motor": return { color: c(p.light ? "#b5652a" : "#e08f4f"), metal: 0.75, rough: 0.35, opacity: 1 };
      case "cabin": return { color: c(p.light ? "#7d879c" : "#8f99ad"), metal: 0, rough: 0.9, opacity: p.light ? 0.3 : 0.22 };
      case "seat": return { color: c(p.light ? "#3a3f4a" : "#2a2e37"), metal: 0, rough: 0.95, opacity: p.light ? 0.75 : 0.7 };
      case "lamp": return { color: c(p.light ? "#e8eefc" : "#f4f8ff"), metal: 0.1, rough: 0.2, opacity: 1 };
      case "rearlamp": return { color: c(p.light ? "#c0362c" : "#ff5f57"), metal: 0.1, rough: 0.3, opacity: 1 };
      // Glass parts get the glass shader; these values are not used.
      case "shell":
      case "screen": return { color: c(p.accent), metal: 0, rough: 0.5, opacity: 0.2 };
    }
  }

  private makeMaterial(m: Material): THREE.MeshStandardMaterial {
    const s = this.materialColor(m);
    const lamp = m === "lamp" || m === "rearlamp";
    return new THREE.MeshStandardMaterial({
      color: s.color,
      metalness: s.metal,
      roughness: s.rough,
      emissive: s.color,
      emissiveIntensity: lamp ? 0.9 : this.palette.light ? 0.05 : 0.16,
      transparent: true,
      opacity: s.opacity,
      depthWrite: s.opacity >= 1,
    });
  }

  // The glass of the body for parts of a motorcycle's bodywork: the fresnel
  // rim without the window graphics of a car; a windscreen darker.
  private glassMaterial(window: boolean): THREE.ShaderMaterial {
    const pal = this.palette;
    const tint = pal.light ? new THREE.Color("#5f7196") : new THREE.Color(pal.accent).lerp(new THREE.Color(pal.sky), 0.45).multiplyScalar(0.55);
    const rim = pal.light ? new THREE.Color("#2b3b63") : new THREE.Color(pal.sky).lerp(new THREE.Color("#ffffff"), 0.25);
    const m = new THREE.ShaderMaterial({
      vertexShader: GLASS_VERTEX,
      fragmentShader: GLASS_FRAGMENT,
      transparent: true,
      depthWrite: false,
      clipping: true,
      side: THREE.DoubleSide,
      uniforms: {
        uTint: { value: tint },
        uRim: { value: rim },
        uWindow: { value: pal.light ? new THREE.Color("#22304f") : new THREE.Color("#03060c") },
        uSky: { value: pal.light ? new THREE.Color("#f2f5fb") : new THREE.Color("#2a3552") },
        uGround: { value: pal.light ? new THREE.Color("#8a93a8") : new THREE.Color("#06080d") },
        uOpacity: { value: pal.light ? 0.16 : 0.13 },
        uWindowAlpha: { value: pal.light ? 0.36 : 0.42 },
        uFade: { value: 1 },
        uBelt: { value: 0 },
        uBeltRear: { value: 0 },
        uRear: { value: -1 },
        uLength: { value: 2 },
        uCowl: { value: 0 },
        uRoofFront: { value: 0 },
        uRoofRear: { value: 0 },
        uTail: { value: 0 },
        uDlo: { value: new THREE.Vector4() },
        uPillars: { value: new THREE.Vector4(-1, -1, -1, -1) },
        uPillarW: { value: new THREE.Vector4() },
        uKink: { value: 0 },
        uGraphic: { value: 0 },
        uWindowAll: { value: window ? 1 : 0 },
      },
    });
    this.glass.push(m);
    return m;
  }

  private partGeometry(part: Part): THREE.BufferGeometry {
    let geo: THREE.BufferGeometry;
    if (part.shape === "box") {
      const [x, y, z] = part.size;
      geo = new RoundedBoxGeometry(x, y, z, 3, Math.min(x, y, z) * (part.round ?? 0.22));
    } else if (part.shape === "ellipsoid") {
      geo = new THREE.SphereGeometry(0.5, 28, 18).scale(...part.size);
    } else {
      const [len, d] = part.size;
      geo = new THREE.CylinderGeometry(d / 2, d / 2, len, 28);
      if (part.axis === "x") geo.rotateZ(Math.PI / 2);
      else if (part.axis === "z") geo.rotateX(Math.PI / 2);
    }
    if (part.tilt) geo.rotateZ(part.tilt);
    return geo;
  }

  private addPart(part: Part): void {
    const geo = this.partGeometry(part);
    if (part.material === "shell" || part.material === "screen") {
      // Bodywork: glass, seen through, not picked.
      const top = new Float32Array(geo.getAttribute("position").count).fill(9);
      geo.setAttribute("aTop", new THREE.BufferAttribute(top, 1));
      const mesh = new THREE.Mesh(geo, this.glassMaterial(part.material === "screen"));
      mesh.position.set(...part.center);
      mesh.renderOrder = part.material === "screen" ? 11 : 10;
      mesh.name = part.id;
      this.layerGroups.get(part.layer)!.add(mesh);
      this.shells.push(mesh);
      return;
    }
    const mat = this.makeMaterial(part.material);
    const mesh = new THREE.Mesh(geo, mat);
    mesh.position.set(...part.center);
    mesh.name = part.id;
    mesh.userData.pick = part.id;
    this.layerGroups.get(part.layer)!.add(mesh);
    this.partMeshes.set(part.id, mesh);
    this.partInfo.set(part.id, { zones: part.zones, baseOpacity: mat.opacity, color: mat.color.clone() });
    if (part.explode) this.explodeMeshes.push({ mesh, home: mesh.position.clone(), offset: v3(part.explode) });
  }

  // A tube and its branches as one mesh.
  private addTube(tube: Tube): void {
    const strands = [tube.points, ...(tube.branches ?? [])].filter((s) => s.length >= 2);
    const geos = strands.map((s) => {
      const curve = pathCurve(s, 0.08);
      return new THREE.TubeGeometry(curve, Math.max(12, Math.round(curve.getLength() * 50)), tube.radius, 8, false);
    });
    if (!geos.length) return;
    const mat = this.makeMaterial(tube.material);
    const mesh = new THREE.Mesh(merged(geos), mat);
    mesh.name = tube.id;
    mesh.userData.pick = tube.id;
    this.layerGroups.get(tube.layer)!.add(mesh);
    this.partMeshes.set(tube.id, mesh);
    this.partInfo.set(tube.id, { zones: tube.zones, baseOpacity: mat.opacity, color: mat.color.clone() });
    if (tube.explode) this.explodeMeshes.push({ mesh, home: new THREE.Vector3(), offset: v3(tube.explode) });
  }

  // The control units and bus lines. Cheap enough to call on every step of
  // a scan: the lines of one bus are one mesh.
  setNetwork(layout: CarLayout): void {
    this.layout = layout;
    for (const child of [...this.networkGroup.children]) {
      this.networkGroup.remove(child);
      disposeTree(child);
    }
    this.ecuMeshes.clear();
    this.ecuNodes.clear();
    this.routes.clear();
    this.diagRoute = null;
    const pal = this.palette;
    const moto = layout.vehicle === "moto";

    // The socket with the adapter in it; on a motorcycle the maker's plug,
    // the adapter cable and the 16-pin socket at its end.
    const socketMat = new THREE.MeshStandardMaterial({ color: pal.light ? 0x2b2f38 : 0x3a4150, roughness: 0.6, emissive: new THREE.Color(pal.bus.diagnostic), emissiveIntensity: 0.35 });
    const ad = layout.adapter;
    const dir = v3(ad.dir).normalize();
    const along = new THREE.Quaternion().setFromUnitVectors(new THREE.Vector3(1, 0, 0), dir);
    const socket = new THREE.Mesh(new RoundedBoxGeometry(0.03, 0.045, 0.06, 2, 0.008), socketMat);
    socket.position.set(...ad.socket);
    socket.quaternion.copy(along);
    socket.userData.pick = "obd";
    this.networkGroup.add(socket);
    const adapterMat = new THREE.MeshStandardMaterial({ color: pal.light ? 0x39404d : 0x252a33, roughness: 0.5, metalness: 0.2, emissive: new THREE.Color(pal.accent), emissiveIntensity: 0.12 });
    const adapter = new THREE.Mesh(new RoundedBoxGeometry(0.075, 0.052, 0.04, 2, 0.012), adapterMat);
    adapter.position.copy(v3(ad.socket).add(dir.clone().multiplyScalar(0.05)));
    adapter.quaternion.copy(along);
    adapter.userData.pick = "obd-adapter";
    const led = new THREE.Mesh(new THREE.SphereGeometry(0.006, 8, 6), new THREE.MeshBasicMaterial({ color: new THREE.Color(pal.good), toneMapped: false }));
    led.position.set(0.02, 0.027, 0);
    adapter.add(led);
    this.networkGroup.add(adapter);
    if (ad.cable) {
      const plug = new THREE.Mesh(new RoundedBoxGeometry(0.05, 0.035, 0.04, 2, 0.008), socketMat);
      plug.position.set(...layout.obd);
      plug.userData.pick = "obd";
      this.networkGroup.add(plug);
      const curve = pathCurve(ad.cable, 0.05);
      const cable = new THREE.Mesh(new THREE.TubeGeometry(curve, 48, 0.005, 6, false), new THREE.MeshStandardMaterial({ color: pal.light ? 0x2b2f38 : 0x1c2028, roughness: 0.7 }));
      cable.userData.pick = "obd-cable";
      this.networkGroup.add(cable);
    }

    const byBus = new Map<BusKind, THREE.BufferGeometry[]>();
    for (const r of layout.routes) {
      const curve = pathCurve(r.points, 0.05);
      const geo = new THREE.TubeGeometry(curve, Math.max(16, Math.round(curve.getLength() * 50)), moto ? 0.004 : 0.0065, 6, false);
      byBus.set(r.bus, [...(byBus.get(r.bus) ?? []), geo]);
      if (r.ecu) this.routes.set(r.ecu, { curve, bus: r.bus });
      else this.diagRoute = { curve, bus: r.bus };
    }
    for (const bus of BUSES) {
      const geos = byBus.get(bus);
      if (!geos) continue;
      const mat = new THREE.MeshBasicMaterial({ color: new THREE.Color(pal.bus[bus]), transparent: true, opacity: pal.light ? 0.9 : 0.75, toneMapped: false });
      const mesh = new THREE.Mesh(merged(geos), mat);
      mesh.renderOrder = 2;
      mesh.name = `bus-${bus}`;
      this.networkGroup.add(mesh);
    }

    const unit = moto ? new RoundedBoxGeometry(0.11, 0.045, 0.085, 2, 0.012) : new RoundedBoxGeometry(0.19, 0.065, 0.14, 2, 0.018);
    for (const e of layout.ecus) {
      const mat = new THREE.MeshStandardMaterial({
        color: new THREE.Color(pal.light ? "#e9ecf3" : "#d7dcea").lerp(new THREE.Color(pal.bus[e.bus]), 0.35),
        roughness: 0.35,
        metalness: 0.4,
        emissive: new THREE.Color(pal.bus[e.bus]),
        emissiveIntensity: pal.light ? 0.35 : 0.6,
        transparent: true,
        opacity: e.asking ? 0.45 : 1,
      });
      const mesh = new THREE.Mesh(unit, mat);
      mesh.position.set(...e.position);
      mesh.userData.pick = `ecu:${e.key}`;
      mesh.userData.base = mat.emissive.clone();
      this.networkGroup.add(mesh);
      this.ecuMeshes.set(e.key, mesh);
      this.ecuNodes.set(e.key, e);
    }
    this.applyHighlight();
    this.applyClipping();
    this.dirty = true;
  }

  // A packet from the socket (through the gateway) to a control unit. With
  // reduced motion the unit only lights up briefly.
  sendPacket(ecuKey: string): void {
    const route = this.routes.get(ecuKey);
    if (!route || !this.visible.network) {
      this.flashes.set(ecuKey, performance.now());
      return;
    }
    if (this.reducedMotion) {
      this.flashes.set(ecuKey, performance.now());
      this.dirty = true;
      return;
    }
    // At most two packets on their way to one unit.
    if (this.packets.filter((p) => p.target === ecuKey).length >= 2) return;
    const points: THREE.Vector3[] = [];
    if (this.diagRoute && route.bus !== "kline") points.push(...this.diagRoute.curve.getSpacedPoints(24));
    points.push(...route.curve.getSpacedPoints(Math.max(16, Math.round(route.curve.getLength() * 30))));
    const curve = new THREE.CatmullRomCurve3(points, false, "centripetal", 0);
    const color = new THREE.Color(this.palette.bus[route.bus]);
    const mesh = new THREE.Mesh(new THREE.SphereGeometry(0.018, 12, 8), new THREE.MeshBasicMaterial({ color: color.clone().lerp(new THREE.Color("#ffffff"), 0.5), toneMapped: false }));
    const glow = new THREE.Sprite(new THREE.SpriteMaterial({ map: this.glow, color, transparent: true, depthWrite: false, blending: THREE.AdditiveBlending, toneMapped: false }));
    glow.scale.setScalar(0.13);
    mesh.add(glow);
    mesh.renderOrder = 20;
    this.packetGroup.add(mesh);
    this.packets.push({ curve, length: curve.getLength(), t: 0, mesh, glow, target: ecuKey });
  }

  // --- State ---------------------------------------------------------------------------

  setLayers(visible: Record<Layer, boolean>): void {
    this.visible = { ...visible };
    this.applyLayers();
  }

  private applyLayers(): void {
    for (const layer of LAYERS) this.layerGroups.get(layer)!.visible = this.visible[layer];
    this.dirty = true;
  }

  setHighlight(h: Highlight): void {
    this.highlight = h;
    this.applyHighlight();
  }

  private applyHighlight(): void {
    const h = this.highlight;
    const hit = new Set(h.zones);
    const focus = h.focusZone;
    const anyFocus = focus !== null || h.focusEcu !== null;
    for (const [id, mesh] of this.partMeshes) {
      const info = this.partInfo.get(id)!;
      const mat = mesh.material as THREE.MeshStandardMaterial;
      const affected = info.zones.some((z) => hit.has(z));
      const focused = focus !== null && info.zones.includes(focus);
      mesh.userData.affected = affected;
      mat.opacity = anyFocus && !focused ? Math.min(info.baseOpacity, 0.28) : info.baseOpacity;
      mat.depthWrite = mat.opacity >= 1;
      if (!affected) {
        mat.emissive.copy(info.color);
        mat.emissiveIntensity = this.palette.light ? 0.05 : 0.16;
      }
    }
    for (const g of this.wheelGroup.children) g.userData.affected = hit.has(CarZone.Wheels);
    for (const m of this.glass) m.uniforms.uFade!.value = anyFocus ? 0.55 : 1;
    for (const [key, mesh] of this.ecuMeshes) {
      const mat = mesh.material as THREE.MeshStandardMaterial;
      const node = this.ecuNodes.get(key)!;
      const dim = anyFocus && h.focusEcu !== key && !(focus === CarZone.Ecu && node.kind === 0) && focus !== CarZone.Network;
      mat.opacity = node.asking ? 0.45 : dim ? 0.35 : 1;
      mesh.scale.setScalar(h.selectedEcu === key || h.focusEcu === key ? 1.35 : 1);
    }
    this.dirty = true;
  }

  setLabels(labels: XrayLabel[]): void {
    const keep = new Set(labels.map((l) => l.id));
    for (const [id, entry] of this.labels) {
      if (!keep.has(id)) {
        entry.el.remove();
        this.labels.delete(id);
      }
    }
    for (const l of labels) {
      let entry = this.labels.get(l.id);
      if (!entry) {
        const el = document.createElement("div");
        el.className = "xray-label";
        this.overlay.appendChild(el);
        entry = { el, data: l };
        this.labels.set(l.id, entry);
      }
      entry.data = l;
      entry.el.dataset.tone = l.tone ?? "";
      if (l.bus) entry.el.style.setProperty("--tone", `var(--bus-${l.bus})`);
      else entry.el.style.removeProperty("--tone");
      const title = l.title;
      const value = l.value ?? "";
      if (entry.el.dataset.title !== title || entry.el.dataset.value !== value) {
        entry.el.dataset.title = title;
        entry.el.dataset.value = value;
        entry.el.replaceChildren();
        entry.w = undefined;
        entry.h = undefined;
        const t = document.createElement("span");
        t.className = "xray-label-title";
        t.textContent = title;
        entry.el.appendChild(t);
        if (value) {
          const v = document.createElement("span");
          v.className = "xray-label-value";
          v.textContent = value;
          entry.el.appendChild(v);
        }
      }
    }
    this.dirty = true;
  }

  setPalette(palette: XrayPalette): void {
    this.palette = palette;
    if (this.layout) this.setCar(this.layout, true);
  }

  setActive(active: boolean): void {
    if (this.active === active) return;
    this.active = active;
    if (active && !this.raf) {
      this.dirty = true;
      this.last = 0;
      this.raf = requestAnimationFrame(this.loop);
    }
  }

  // --- Cuts ------------------------------------------------------------------------------

  // The exploded view of the drivetrain: on, the parts move out over half a
  // second (at once with reduced motion).
  setExplode(on: boolean): void {
    this.explodeTarget = on ? 1 : 0;
    if (this.reducedMotion) {
      this.explodeT = this.explodeTarget;
      this.applyExplode();
    }
    this.dirty = true;
  }

  private applyExplode(): void {
    const k = ease(this.explodeT);
    for (const e of this.explodeMeshes) e.mesh.position.copy(e.home).addScaledVector(e.offset, k);
    this.dirty = true;
  }

  // The section plane through the car, null for none.
  setSection(section: Section | null): void {
    this.section = section ? { axis: section.axis, t: Math.min(0.98, Math.max(0.02, section.t)) } : null;
    this.placeSection();
    this.applyClipping();
  }

  // The extent of the car along an axis.
  private extent(axis: "x" | "y" | "z"): [number, number] {
    const hull = this.layout?.hull;
    if (!hull) return [-2, 2];
    if (axis === "x") return [hull.rear, hull.front];
    if (axis === "y") return [0, hull.height];
    return [-hull.width / 2, hull.width / 2];
  }

  // The plane keeps what lies behind it (towards the rear, the passenger
  // side or the floor), and its frame shows where it cuts.
  private placeSection(): void {
    for (const child of [...this.sectionGroup.children]) {
      this.sectionGroup.remove(child);
      disposeTree(child);
    }
    this.sectionHandle = null;
    const s = this.section;
    const hull = this.layout?.hull;
    if (!s || !hull) {
      this.clip.length = 0;
      this.sectionGroup.visible = false;
      this.dirty = true;
      return;
    }
    const [lo, hi] = this.extent(s.axis);
    const at = lo + (hi - lo) * s.t;
    const normal = s.axis === "x" ? new THREE.Vector3(-1, 0, 0) : s.axis === "y" ? new THREE.Vector3(0, -1, 0) : new THREE.Vector3(0, 0, -1);
    this.clipPlane.set(normal, at);
    this.clip.length = 0;
    this.clip.push(this.clipPlane);
    // The frame: a little larger than the car's cross-section there.
    const L = hull.length * 1.08;
    const W = hull.width * 1.15;
    const H = hull.height * 1.12;
    const [w, h] = s.axis === "x" ? [W, H] : s.axis === "y" ? [L, W] : [L, H];
    const color = new THREE.Color(this.palette.accent);
    const quad = new THREE.Mesh(
      new THREE.PlaneGeometry(w, h),
      new THREE.MeshBasicMaterial({ color, transparent: true, opacity: this.palette.light ? 0.08 : 0.07, side: THREE.DoubleSide, depthWrite: false, toneMapped: false }),
    );
    quad.renderOrder = 30;
    const frame = new THREE.LineSegments(
      new THREE.EdgesGeometry(new THREE.PlaneGeometry(w, h)),
      new THREE.LineBasicMaterial({ color, transparent: true, opacity: 0.85, toneMapped: false }),
    );
    frame.renderOrder = 31;
    const g = new THREE.Group();
    g.add(quad, frame);
    if (s.axis === "x") {
      g.rotation.y = Math.PI / 2;
      g.position.set(at, hull.height / 2, 0);
    } else if (s.axis === "y") {
      g.rotation.x = -Math.PI / 2;
      g.position.set(0, at, 0);
    } else {
      g.position.set(0, hull.height / 2, at);
    }
    this.sectionGroup.add(g);
    this.sectionHandle = quad;
    this.sectionGroup.visible = true;
    this.dirty = true;
  }

  // Every material of the car and the network shares the plane list, so
  // switching the cut on and off needs no walk over the scene afterwards.
  private applyClipping(): void {
    const roots: THREE.Object3D[] = [...this.layerGroups.values(), this.wheelGroup];
    for (const root of roots) {
      root.traverse((o) => {
        const mat = (o as THREE.Mesh).material as THREE.Material | THREE.Material[] | undefined;
        if (!mat) return;
        for (const m of Array.isArray(mat) ? mat : [mat]) m.clippingPlanes = this.clip;
      });
    }
    this.dirty = true;
  }

  private onSectionDown = (e: PointerEvent) => {
    if (!this.section || !this.sectionHandle) return;
    const rect = this.renderer.domElement.getBoundingClientRect();
    const ndc = new THREE.Vector2(((e.clientX - rect.left) / rect.width) * 2 - 1, -((e.clientY - rect.top) / rect.height) * 2 + 1);
    this.raycaster.setFromCamera(ndc, this.camera);
    if (!this.raycaster.intersectObject(this.sectionHandle, false).length) return;
    // The plane takes the press: the view does not turn.
    this.dragging = true;
    this.controls.enabled = false;
    this.renderer.domElement.setPointerCapture(e.pointerId);
    this.renderer.domElement.addEventListener("pointermove", this.onSectionMove);
    this.renderer.domElement.addEventListener("pointerup", this.onSectionUp);
    e.stopPropagation();
  };

  private onSectionMove = (e: PointerEvent) => {
    const s = this.section;
    if (!this.dragging || !s) return;
    const rect = this.renderer.domElement.getBoundingClientRect();
    const ndc = new THREE.Vector2(((e.clientX - rect.left) / rect.width) * 2 - 1, -((e.clientY - rect.top) / rect.height) * 2 + 1);
    this.raycaster.setFromCamera(ndc, this.camera);
    // The point of the axis through the car nearest to the ray.
    const ray = this.raycaster.ray;
    const axis = s.axis === "x" ? new THREE.Vector3(1, 0, 0) : s.axis === "y" ? new THREE.Vector3(0, 1, 0) : new THREE.Vector3(0, 0, 1);
    const origin = new THREE.Vector3(0, (this.layout?.hull.height ?? 1.4) / 2, 0);
    const w0 = origin.clone().sub(ray.origin);
    const b = axis.dot(ray.direction);
    const d = axis.dot(w0);
    const f = ray.direction.dot(w0);
    const denom = 1 - b * b;
    if (denom < 1e-4) return;
    const along = (b * f - d) / denom;
    const pos = (s.axis === "x" ? origin.x : s.axis === "y" ? origin.y : origin.z) + along;
    const [lo, hi] = this.extent(s.axis);
    const t = Math.min(0.98, Math.max(0.02, (pos - lo) / (hi - lo)));
    this.setSection({ axis: s.axis, t });
    this.events.onSection?.(t);
  };

  private onSectionUp = (e: PointerEvent) => {
    this.dragging = false;
    this.controls.enabled = true;
    const canvas = this.renderer.domElement;
    if (canvas.hasPointerCapture(e.pointerId)) canvas.releasePointerCapture(e.pointerId);
    canvas.removeEventListener("pointermove", this.onSectionMove);
    canvas.removeEventListener("pointerup", this.onSectionUp);
    // No click on what lies under the plane.
    this.downAt = null;
  };

  // The frame counter: on, the scene draws every frame and reports the
  // frame time, the time of the draw call, draw calls and triangles.
  setStats(on: boolean): void {
    this.statsOn = on;
    this.statsFrames = [];
    this.statsRender = [];
    this.dirty = true;
  }

  // --- Camera ----------------------------------------------------------------------------

  flyTo(point: Vec3, distance?: number): void {
    const target = v3(point);
    const dir = this.camera.position.clone().sub(this.controls.target);
    if (dir.lengthSq() < 1e-6) dir.set(1, 1, 1);
    dir.normalize();
    // Look from well above, so the wheels and the body sides do not stand
    // between the camera and a part low in the car.
    dir.y = Math.max(dir.y, 0.8);
    dir.normalize();
    const L = this.layout?.hull.length ?? 4.5;
    const d = distance ?? Math.max(3, L * 0.98);
    this.animateTo(target.clone().add(dir.multiplyScalar(d)), target);
  }

  resetView(): void {
    this.animateTo(this.home.position.clone(), this.home.target.clone());
  }

  // A straight view from one side, for comparing silhouettes (the
  // screenshot option's "|side"); "rear" looks from behind the car.
  viewFrom(side: "side" | "rear" | "front"): void {
    const L = this.layout?.hull.length ?? 4.5;
    const target = this.home.target.clone();
    const distance = ((L * 0.5) / Math.tan(THREE.MathUtils.degToRad(this.camera.fov / 2))) * 1.25;
    this.tween = null;
    const dir = side === "side" ? new THREE.Vector3(0, 0.08, 1) : side === "rear" ? new THREE.Vector3(-1, 0.25, 0.35) : new THREE.Vector3(1, 0.25, 0.35);
    this.camera.position.copy(dir.normalize().multiplyScalar(distance).add(target));
    this.controls.target.copy(target);
    this.controls.update();
    this.dirty = true;
  }

  private animateTo(position: THREE.Vector3, target: THREE.Vector3): void {
    if (this.reducedMotion) {
      this.camera.position.copy(position);
      this.controls.target.copy(target);
      this.controls.update();
      this.dirty = true;
      return;
    }
    this.tween = {
      fromPos: this.camera.position.clone(),
      toPos: position,
      fromTarget: this.controls.target.clone(),
      toTarget: target,
      start: performance.now(),
      duration: 1000,
    };
  }

  // --- Input -------------------------------------------------------------------------------

  private onKeyDown = (e: KeyboardEvent) => {
    const step = 0.09;
    let handled = true;
    const pan = e.shiftKey || e.ctrlKey || e.metaKey;
    switch (e.key) {
      case "ArrowLeft":
        if (pan) this.controls.pan(40, 0);
        else this.controls.rotateLeft(step);
        break;
      case "ArrowRight":
        if (pan) this.controls.pan(-40, 0);
        else this.controls.rotateLeft(-step);
        break;
      case "ArrowUp":
        if (pan) this.controls.pan(0, 40);
        else this.controls.rotateUp(step * 0.7);
        break;
      case "ArrowDown":
        if (pan) this.controls.pan(0, -40);
        else this.controls.rotateUp(-step * 0.7);
        break;
      case "+":
      case "=":
        this.controls.dollyIn(1.15);
        break;
      case "-":
      case "_":
        this.controls.dollyOut(1.15);
        break;
      case "0":
      case "Home":
        this.resetView();
        break;
      default:
        handled = false;
    }
    if (handled) {
      e.preventDefault();
      this.controls.update();
      this.dirty = true;
    }
  };

  private onPointerMove = (e: PointerEvent) => {
    const rect = this.renderer.domElement.getBoundingClientRect();
    this.pointer = { x: ((e.clientX - rect.left) / rect.width) * 2 - 1, y: -((e.clientY - rect.top) / rect.height) * 2 + 1 };
  };

  private onPointerLeave = () => {
    this.pointer = null;
    this.setHovered(null);
  };

  private onPointerDown = (e: PointerEvent) => {
    this.downAt = { x: e.clientX, y: e.clientY };
  };

  private onPointerUp = (e: PointerEvent) => {
    if (!this.downAt) return;
    const moved = Math.hypot(e.clientX - this.downAt.x, e.clientY - this.downAt.y);
    this.downAt = null;
    if (moved > 5) return;
    this.onPointerMove(e);
    this.events.onPick?.(this.pick());
  };

  private pick(): string | null {
    if (!this.pointer) return null;
    this.raycaster.setFromCamera(new THREE.Vector2(this.pointer.x, this.pointer.y), this.camera);
    const candidates: THREE.Object3D[] = [];
    if (this.visible.network) {
      candidates.push(...this.ecuMeshes.values());
      // The socket, the adapter in it and a motorcycle's adapter cable.
      for (const o of this.networkGroup.children) if (String(o.userData.pick ?? "").startsWith("obd")) candidates.push(o);
    }
    for (const [, mesh] of this.partMeshes) {
      if (mesh.parent?.visible && (mesh.material as THREE.MeshStandardMaterial).opacity > 0.25) candidates.push(mesh);
    }
    const hits = this.raycaster.intersectObjects(candidates, false);
    // Control units win over the parts they sit on.
    const ecu = hits.find((h) => String(h.object.userData.pick).startsWith("ecu:"));
    const hit = ecu ?? hits[0];
    return hit ? String(hit.object.userData.pick) : null;
  }

  private setHovered(id: string | null): void {
    if (id === this.hovered) return;
    this.hovered = id;
    this.renderer.domElement.style.cursor = id ? "pointer" : "";
    this.events.onHover?.(id);
  }

  // --- Frame ----------------------------------------------------------------------------------

  private fit(): void {
    const w = Math.max(1, this.container.clientWidth);
    const h = Math.max(1, this.container.clientHeight);
    this.renderer.setSize(w, h, false);
    this.camera.aspect = w / h;
    this.camera.updateProjectionMatrix();
    this.dirty = true;
  }

  private loop = (now: number) => {
    if (this.disposed) return;
    if (!this.active) {
      this.raf = 0;
      return;
    }
    this.raf = requestAnimationFrame(this.loop);
    const dt = this.last ? Math.min(0.05, (now - this.last) / 1000) : 0;
    this.last = now;

    if (this.tween) {
      const t = Math.min(1, Math.max(0, (performance.now() - this.tween.start) / this.tween.duration));
      const k = ease(t);
      this.camera.position.lerpVectors(this.tween.fromPos, this.tween.toPos, k);
      this.controls.target.lerpVectors(this.tween.fromTarget, this.tween.toTarget, k);
      if (t >= 1) this.tween = null;
      this.dirty = true;
    }
    if (this.controls.update()) this.dirty = true;

    // The drivetrain moving apart or back together.
    if (this.explodeT !== this.explodeTarget) {
      const step = dt / 0.6;
      this.explodeT = this.explodeTarget > this.explodeT ? Math.min(this.explodeTarget, this.explodeT + step) : Math.max(this.explodeTarget, this.explodeT - step);
      this.applyExplode();
    }

    // Packets.
    if (this.packets.length > 0) {
      for (const p of [...this.packets]) {
        p.t += (dt * 2.2) / Math.max(0.5, p.length);
        if (p.t >= 1) {
          this.packetGroup.remove(p.mesh);
          disposeTree(p.mesh);
          this.packets.splice(this.packets.indexOf(p), 1);
          this.flashes.set(p.target, performance.now());
          continue;
        }
        p.mesh.position.copy(p.curve.getPointAt(p.t));
      }
      this.dirty = true;
    }

    // Pulsing faults and flashing units.
    const pulse = this.reducedMotion ? 0.75 : 0.55 + 0.45 * Math.sin(now / 260);
    const bad = new THREE.Color(this.palette.bad);
    let animating = false;
    for (const [, mesh] of this.partMeshes) {
      if (!mesh.userData.affected) continue;
      const mat = mesh.material as THREE.MeshStandardMaterial;
      mat.emissive.copy(bad);
      mat.emissiveIntensity = 0.35 + pulse * 0.9;
      animating = true;
    }
    for (const g of this.wheelGroup.children) {
      if (!g.userData.affected) continue;
      animating = true;
      g.traverse((o) => {
        const mat = (o as THREE.Mesh).material as THREE.MeshStandardMaterial | undefined;
        if (mat && "emissive" in mat && o.userData.glow) {
          mat.emissive.copy(bad);
          mat.emissiveIntensity = 0.2 + pulse * 0.6;
        }
      });
    }
    for (const [key, mesh] of this.ecuMeshes) {
      const node = this.ecuNodes.get(key)!;
      const mat = mesh.material as THREE.MeshStandardMaterial;
      const flash = this.flashes.get(key);
      const since = flash ? Math.max(0, performance.now() - flash) / 1000 : 9;
      if (since < 0.6) {
        mat.emissive.copy(new THREE.Color(this.palette.bus[node.bus])).lerp(new THREE.Color("#ffffff"), 0.6 * (1 - since / 0.6));
        mat.emissiveIntensity = 1.6 - since;
        animating = true;
      } else if (node.faulty) {
        mat.emissive.copy(bad);
        mat.emissiveIntensity = 0.5 + pulse * 0.9;
        animating = true;
      } else if (node.asking) {
        mat.emissive.copy(new THREE.Color(this.palette.accent));
        mat.emissiveIntensity = 0.4 + pulse * 0.6;
        animating = true;
      } else {
        mat.emissive.copy(mesh.userData.base as THREE.Color);
        mat.emissiveIntensity = this.palette.light ? 0.35 : 0.6;
      }
    }
    if (animating && !this.reducedMotion) this.dirty = true;

    // Hover, at most once per frame.
    if (this.pointer && !this.downAt) this.setHovered(this.pick());

    if (this.statsOn) this.dirty = true;
    if (!this.dirty) return;
    this.dirty = false;
    const before = performance.now();
    this.renderer.render(this.scene, this.camera);
    this.placeLabels();
    if (this.statsOn) this.countFrame(now, performance.now() - before);
  };

  private countFrame(now: number, renderMs: number): void {
    this.statsFrames.push(now);
    this.statsRender.push(renderMs);
    if (now - this.statsAt < 500 || this.statsFrames.length < 3) return;
    this.statsAt = now;
    const frames = this.statsFrames;
    const span = frames[frames.length - 1]! - frames[0]!;
    const frameMs = span / (frames.length - 1);
    const info = this.renderer.info.render;
    this.events.onStats?.({
      fps: 1000 / frameMs,
      frameMs,
      renderMs: this.statsRender.reduce((a, b) => a + b, 0) / this.statsRender.length,
      calls: info.calls,
      triangles: info.triangles,
    });
    this.statsFrames = [now];
    this.statsRender = [];
  }

  private placeLabels(): void {
    const w = this.container.clientWidth;
    const h = this.container.clientHeight;
    const v = new THREE.Vector3();
    const shown: { entry: { el: HTMLDivElement; w?: number; h?: number }; x: number; y: number }[] = [];
    // First every write of the visibility, then the reads of the sizes that
    // are not known yet, then the positions: one layout pass at most.
    for (const [, entry] of this.labels) {
      v.set(...entry.data.anchor).project(this.camera);
      const visible = v.z < 1 && Math.abs(v.x) < 1.1 && Math.abs(v.y) < 1.1;
      const display = visible ? "" : "none";
      if (entry.el.style.display !== display) entry.el.style.display = display;
      if (visible) shown.push({ entry, x: (v.x * 0.5 + 0.5) * w, y: (-v.y * 0.5 + 0.5) * h });
    }
    for (const s of shown) {
      if (s.entry.w === undefined) {
        s.entry.w = s.entry.el.offsetWidth;
        s.entry.h = s.entry.el.offsetHeight;
      }
    }
    const placed = shown.map((s) => ({ el: s.entry.el, x: s.x, y: s.y, w: s.entry.w ?? 0, h: s.entry.h ?? 0 }));
    // Labels that would cover each other move up, the lower one first.
    placed.sort((a, b) => b.y - a.y);
    for (let i = 0; i < placed.length; i++) {
      const a = placed[i]!;
      for (let pass = 0; pass < 3; pass++) {
        let moved = false;
        for (let j = 0; j < i; j++) {
          const b = placed[j]!;
          const overlapX = Math.abs(a.x - b.x) < (a.w + b.w) / 2 + 4;
          const overlapY = a.y > b.y - b.h - 4 && a.y - a.h < b.y + 4;
          if (overlapX && overlapY) {
            a.y = b.y - b.h - 6;
            moved = true;
          }
        }
        if (!moved) break;
      }
      a.el.style.transform = `translate(${a.x.toFixed(1)}px, ${a.y.toFixed(1)}px)`;
    }
  }

  // Screen position of a point, for overlays of the page.
  project(point: Vec3): { x: number; y: number } {
    const v = v3(point).project(this.camera);
    return { x: (v.x * 0.5 + 0.5) * this.container.clientWidth, y: (-v.y * 0.5 + 0.5) * this.container.clientHeight };
  }

  // Replaces the generated hull by a 3D model (glTF or GLB) the user owns:
  // turned, scaled and moved onto the car (shell.ts), drawn in the same
  // glass as the generated body, its glass parts darker. The drivetrain, the
  // control units and the lines stay as the layout put them. The model's own
  // wheels replace the generated ones when it has any. files: the buffers
  // and pictures a .gltf names next to itself, by name.
  async loadShell(data: ArrayBuffer, options: { turned: boolean; files?: Map<string, Blob> }): Promise<ShellFit> {
    const { GLTFLoader } = await import("three/examples/jsm/loaders/GLTFLoader.js");
    const manager = new THREE.LoadingManager();
    const urls = new Map<string, string>();
    for (const [name, blob] of options.files ?? []) urls.set(name, URL.createObjectURL(blob));
    manager.setURLModifier((url) => {
      const name = decodeURIComponent(url.replace(/^.*[\\/]/, ""));
      return urls.get(name) ?? urls.get(decodeURIComponent(url)) ?? url;
    });
    try {
      const gltf = await new GLTFLoader(manager).parseAsync(data, "");
      this.clearShell();
      const inner = gltf.scene;
      const outer = new THREE.Group();
      outer.name = "shell";
      outer.add(inner);
      this.shell = { outer, inner, turned: options.turned, fit: null };
      this.placeShell();
      return this.shell.fit!;
    } finally {
      for (const url of urls.values()) URL.revokeObjectURL(url);
    }
  }

  // Turns a loaded model around (it faced backwards).
  turnShell(turned: boolean): ShellFit | null {
    if (!this.shell) return null;
    this.shell.turned = turned;
    this.placeShell();
    return this.shell.fit;
  }

  hasShell(): boolean {
    return this.shell !== null;
  }

  // Back to the generated body.
  clearShell(): void {
    if (!this.shell) return;
    this.shell.outer.parent?.remove(this.shell.outer);
    disposeTree(this.shell.outer);
    this.shell = null;
    if (this.layout) this.setCar(this.layout, true);
  }

  private placeShell(): void {
    const shell = this.shell;
    const layout = this.layout;
    if (!shell || !layout) return;
    const { inner, outer } = shell;
    // Measure the model as it is, then turned.
    inner.rotation.set(0, 0, 0);
    inner.position.set(0, 0, 0);
    inner.scale.set(1, 1, 1);
    outer.position.set(0, 0, 0);
    outer.scale.set(1, 1, 1);
    outer.updateMatrixWorld(true);
    const raw = new THREE.Box3().setFromObject(inner);
    const size = raw.getSize(new THREE.Vector3());
    inner.rotation.y = orientation([size.x, size.y, size.z], shell.turned);
    outer.updateMatrixWorld(true);
    const box = new THREE.Box3().setFromObject(inner);
    const bounds: Bounds = { min: [box.min.x, box.min.y, box.min.z], max: [box.max.x, box.max.y, box.max.z] };
    const points: Vec3[] = [];
    const v = new THREE.Vector3();
    inner.traverse((o) => {
      const mesh = o as THREE.Mesh;
      if (!mesh.isMesh) return;
      const pos = mesh.geometry.getAttribute("position");
      const step = Math.max(1, Math.floor(pos.count / 4000));
      for (let i = 0; i < pos.count; i += step) {
        v.fromBufferAttribute(pos, i).applyMatrix4(mesh.matrixWorld);
        points.push([v.x, v.y, v.z]);
      }
    });
    const axles = contactAxles(points, bounds);
    const fit = fitShell(bounds, axles, {
      length: layout.hull.length,
      wheelbase: layout.frontAxle - layout.rearAxle,
      frontAxle: layout.frontAxle,
      rearAxle: layout.rearAxle,
    }, inner.rotation.y);
    // The pivot is in turned coordinates: the inner group turns first, then
    // moves the pivot to its origin; the outer one scales and places it.
    inner.position.set(-fit.pivot[0], -fit.pivot[1], -fit.pivot[2]);
    outer.scale.set(fit.scale * fit.stretchX, fit.scale, fit.scale);
    outer.position.set(...fit.position);
    shell.fit = fit;
    this.applyShellLook(axles !== null);
  }

  // Glass for the model, darker where the model has glass; the generated
  // body and, when the model brings its own, the generated wheels go.
  private applyShellLook(ownWheels: boolean): void {
    const shell = this.shell;
    if (!shell) return;
    const body = this.layerGroups.get("body")!;
    for (const child of [...body.children]) {
      if (child.name === "hull" || child.name === "outline" || child.name === "lamps") {
        body.remove(child);
        disposeTree(child);
      }
    }
    if (!shell.outer.parent) body.add(shell.outer);
    this.wheelGroup.visible = !ownWheels;
    this.dirty = true;
    const front = this.glass.find((m) => m.side === THREE.FrontSide);
    // Already dressed (turned around): the materials stay.
    if (!front) return;
    // The hull's two materials go with the hull; the model gets its own.
    this.glass = [];
    const glassFor = (window: boolean) => {
      const m = front.clone();
      m.side = THREE.DoubleSide;
      m.uniforms.uGraphic!.value = 0;
      m.uniforms.uWindowAll!.value = window ? 1 : 0;
      m.uniforms.uOpacity!.value = this.palette.light ? 0.06 : 0.045;
      this.glass.push(m);
      return m;
    };
    const plain = glassFor(false);
    const windows = glassFor(true);
    front.dispose();
    const tyre = new THREE.MeshStandardMaterial({ color: this.palette.light ? 0x2a2d34 : 0x15171c, roughness: 0.85, metalness: 0.05 });
    shell.inner.traverse((o) => {
      const mesh = o as THREE.Mesh;
      if (!mesh.isMesh) return;
      const old = (Array.isArray(mesh.material) ? mesh.material[0] : mesh.material) as THREE.Material | undefined;
      const name = `${mesh.name} ${old?.name ?? ""}`;
      const glass = looksLikeGlass(name, !!old?.transparent, old?.opacity ?? 1);
      // The glass needs normals; models without them get computed ones.
      if (!mesh.geometry.getAttribute("normal")) mesh.geometry.computeVertexNormals();
      if (!mesh.geometry.getAttribute("aTop")) {
        const count = mesh.geometry.getAttribute("position").count;
        mesh.geometry.setAttribute("aTop", new THREE.BufferAttribute(new Float32Array(count).fill(9), 1));
      }
      if (old && old !== plain && old !== windows && old !== tyre) old.dispose();
      const wheel = !glass && looksLikeWheel(name);
      mesh.material = wheel ? tyre : glass ? windows : plain;
      mesh.renderOrder = wheel ? 0 : glass ? 11 : 10;
    });
    this.applyHighlight();
  }

  dispose(): void {
    this.disposed = true;
    cancelAnimationFrame(this.raf);
    this.resize.disconnect();
    const canvas = this.renderer.domElement;
    canvas.removeEventListener("pointermove", this.onPointerMove);
    canvas.removeEventListener("pointerleave", this.onPointerLeave);
    canvas.removeEventListener("pointerdown", this.onPointerDown);
    canvas.removeEventListener("pointerup", this.onPointerUp);
    this.container.removeEventListener("keydown", this.onKeyDown);
    this.container.removeEventListener("pointerdown", this.onSectionDown, true);
    canvas.removeEventListener("pointermove", this.onSectionMove);
    canvas.removeEventListener("pointerup", this.onSectionUp);
    this.controls.dispose();
    disposeTree(this.scene);
    this.glow.dispose();
    this.scene.environment?.dispose();
    this.renderer.dispose();
    canvas.remove();
    this.overlay.remove();
  }
}

// Several geometries as one mesh (fewer draw calls on a weak graphics
// chip). They are all indexed and have the same attributes.
function merged(geos: THREE.BufferGeometry[]): THREE.BufferGeometry {
  if (geos.length === 1) return geos[0]!;
  const out = mergeGeometries(geos, false);
  if (!out) return geos[0]!;
  for (const g of geos) g.dispose();
  return out;
}

function disposeTree(root: THREE.Object3D): void {
  root.traverse((o) => {
    const mesh = o as THREE.Mesh;
    if (mesh.geometry) mesh.geometry.dispose();
    const mat = mesh.material as THREE.Material | THREE.Material[] | undefined;
    if (!mat) return;
    for (const m of Array.isArray(mat) ? mat : [mat]) {
      const map = (m as THREE.MeshBasicMaterial).map;
      if (map) map.dispose();
      m.dispose();
    }
  });
}

// Whether this computer can draw WebGL at all (not over some remote
// desktops, not with the GPU blocked).
export function webglAvailable(): boolean {
  try {
    const canvas = document.createElement("canvas");
    const gl = canvas.getContext("webgl2") ?? canvas.getContext("webgl");
    return !!gl;
  } catch {
    return false;
  }
}
