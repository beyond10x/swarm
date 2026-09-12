import {useEffect, useRef} from 'react';
import {PageHeader} from '@beyond10x/docs-system/components';

/**
 * The hero animation: the loop, drawn.
 *
 * What it depicts is what the system is, rather than an abstract field of drifting dots. A token
 * travels a three-station ring — `[loop] → [coordinator] → [goal]` — which is the cycle every swarm
 * is born with. Passing the coordinator station fires a turn: an event burst leaves the coordinator
 * and cascades rightward through the box canvas along connections, depth-capped exactly as the pump
 * is. Arriving at the goal station prints the verdict, "not yet" for four turns and "reached" on the
 * fifth, which completes the goal ring, holds, and restarts the run.
 *
 * Rendering rules this file keeps:
 *
 *   - One `<canvas>`, one `requestAnimationFrame` loop, delta-driven with a clamped step, so a tab
 *     that was backgrounded does not fast-forward the simulation on the frame it wakes up.
 *   - devicePixelRatio-aware backing store, resized from a `ResizeObserver`. No layout is read
 *     inside the frame callback, so there is nothing to thrash.
 *   - Paused by `IntersectionObserver` when off-screen and by `visibilitychange` when the tab is
 *     hidden; a paused animation schedules no frames at all.
 *   - `prefers-reduced-motion` draws one *composed* frame — the same simulation advanced to a turn
 *     caught mid-cascade, then stopped. Not a frozen first frame, and not an empty box.
 *   - Every colour is read from the design-system tokens on the element itself, so both themes are
 *     the house palette and a theme switch redraws rather than needing a reload.
 *
 * Determinism: the React tree rendered here contains no generated values — the canvas is empty
 * markup and every label beside it is literal text — so the server and the client produce identical
 * HTML and there is nothing to mismatch on hydration. The geometry is still derived from a seeded
 * PRNG, as the version this replaced was, so one viewport always yields one picture and the
 * reduced-motion still frame is reproducible rather than being whatever frame happened to land.
 */

const SEED = 0x5eed;
const TURN_SECONDS = 4.2; // one lap of the ring: the 30-second tick, compressed to something watchable
const TURNS_TO_REACH = 5;
const HOLD_SECONDS = 1.8; // the pause on "reached" before the run starts over
const MAX_CASCADE_DEPTH = 3; // the pump caps cascade depth; so does the drawing of it
const PARTICLE_LIMIT = 240;
const EDGE_SPEED = 0.85; // progress per second along one connection
const STILL_FRAME_STEPS = Math.round(TURN_SECONDS * 60 * 1.62); // turn 2, mid-cascade

const BOX_NAMES = ['tool', 'mcp', 'panel', 'queue', 'store', 'hook', 'view', 'probe'];

function mulberry32(seed) {
  return function () {
    seed |= 0;
    seed = (seed + 0x6d2b79f5) | 0;
    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const clamp = (v, lo, hi) => (v < lo ? lo : v > hi ? hi : v);
const smooth = (t) => t * t * (3 - 2 * t);

function roundRect(ctx, x, y, w, h, r) {
  const rr = Math.min(r, w / 2, h / 2);
  ctx.beginPath();
  ctx.moveTo(x + rr, y);
  ctx.arcTo(x + w, y, x + w, y + h, rr);
  ctx.arcTo(x + w, y + h, x, y + h, rr);
  ctx.arcTo(x, y + h, x, y, rr);
  ctx.arcTo(x, y, x + w, y, rr);
  ctx.closePath();
}

/** `#rrggbb` (or any computed colour) at a given alpha, without a colour library. */
function alpha(color, a) {
  const hex = color.trim();
  if (hex.startsWith('#') && (hex.length === 7 || hex.length === 4)) {
    const full =
      hex.length === 4 ? `#${hex[1]}${hex[1]}${hex[2]}${hex[2]}${hex[3]}${hex[3]}` : hex;
    const n = parseInt(full.slice(1), 16);
    return `rgba(${(n >> 16) & 255}, ${(n >> 8) & 255}, ${n & 255}, ${a})`;
  }
  if (hex.startsWith('rgb(')) return hex.replace('rgb(', 'rgba(').replace(')', `, ${a})`);
  return hex;
}

/**
 * The stations, the box field and the connections between them, sized to the space available.
 * Same width and height in, same scene out — the PRNG is seeded per call.
 */
function buildScene(width, height) {
  const rand = mulberry32(SEED);
  const pad = Math.max(14, Math.min(28, width * 0.02));
  const compact = width < 720;

  const ringRx = clamp(width * (compact ? 0.2 : 0.13), 54, 128);
  const ringRy = clamp(height * 0.29, 44, 108);
  const ringCx = pad + ringRx + (compact ? 26 : 40);
  const ringCy = height * 0.5;

  // Stations sit on the ring at thirds, travelled in order: loop → coordinator → goal → loop.
  const stationAt = (u) => {
    const a = Math.PI + u * Math.PI * 2;
    return {x: ringCx + Math.cos(a) * ringRx, y: ringCy + Math.sin(a) * ringRy};
  };
  const stations = [
    {...stationAt(0), label: 'loop', align: 'right'},
    {...stationAt(1 / 3), label: 'coordinator', align: 'top'},
    {...stationAt(2 / 3), label: 'goal', align: 'bottom'},
  ];

  const fieldX0 = ringCx + ringRx + (compact ? 34 : 62);
  const fieldX1 = width - pad;
  const fieldW = Math.max(0, fieldX1 - fieldX0);
  const columns = clamp(Math.floor(fieldW / (compact ? 110 : 148)), 0, 4);
  const boxW = columns ? clamp(fieldW / columns - 26, 62, 116) : 0;
  const boxH = clamp(height * 0.115, 26, 40);

  const boxes = [];
  for (let c = 0; c < columns; c += 1) {
    const bandX = fieldX0 + (fieldW / columns) * c;
    const rows = c === columns - 1 ? 2 : 3;
    const usable = height - pad * 2;
    for (let r = 0; r < rows; r += 1) {
      const slot = usable / rows;
      const jitter = (rand() - 0.5) * slot * 0.28;
      boxes.push({
        col: c,
        x: bandX + (rand() - 0.5) * 8,
        y: pad + slot * r + (slot - boxH) / 2 + jitter,
        w: boxW,
        h: boxH,
        name: BOX_NAMES[boxes.length % BOX_NAMES.length],
      });
    }
  }

  // Connections: the coordinator feeds the first column, and every box wires forward to one or two
  // boxes in the next column. Two boxes connect only where their ports agree, so the graph is a
  // fixed, readable cascade rather than a hairball.
  const edges = [];
  const port = (box, side) => ({
    x: side === 'in' ? box.x : box.x + box.w,
    y: box.y + box.h / 2,
  });
  const coordinator = stations[1];
  boxes.forEach((box, index) => {
    if (box.col === 0) {
      edges.push({from: {x: coordinator.x, y: coordinator.y}, to: port(box, 'in'), target: index});
    }
  });
  for (let c = 0; c < columns - 1; c += 1) {
    const here = boxes.map((b, i) => ({b, i})).filter((e) => e.b.col === c);
    const next = boxes.map((b, i) => ({b, i})).filter((e) => e.b.col === c + 1);
    here.forEach(({b, i}) => {
      const fanOut = next.length > 1 && rand() > 0.45 ? 2 : 1;
      const first = Math.floor(rand() * next.length);
      for (let k = 0; k < fanOut; k += 1) {
        const pick = next[(first + k) % next.length];
        edges.push({from: port(b, 'out'), to: port(pick.b, 'in'), target: pick.i, source: i});
      }
    });
  }

  const outgoing = boxes.map(() => []);
  edges.forEach((edge, index) => {
    if (edge.source !== undefined) outgoing[edge.source].push(index);
  });
  const seeds = edges.map((e, i) => (e.source === undefined ? i : -1)).filter((i) => i >= 0);

  return {
    pad,
    compact,
    width,
    height,
    ring: {cx: ringCx, cy: ringCy, rx: ringRx, ry: ringRy},
    stations,
    boxes,
    edges,
    outgoing,
    seeds,
    stationR: clamp(Math.min(width, height) * 0.026, 9, 15),
    fontLabel: clamp(height * 0.036, 9, 12),
    fontBox: clamp(height * 0.034, 8.5, 11.5),
    fontRead: clamp(height * 0.042, 10, 14),
  };
}

function emptyState() {
  return {
    u: 0,
    turn: 1,
    hold: 0,
    reached: false,
    flash: 0,
    fired: -1,
    verdict: 0,
    particles: [],
    boxAct: [],
    edgeAct: [],
  };
}

export default function HeroBanner({eyebrow, title, tagline, actions, children}) {
  const canvasRef = useRef(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return undefined;
    const ctx = canvas.getContext('2d');
    if (!ctx) return undefined;

    const motionQuery = window.matchMedia('(prefers-reduced-motion: reduce)');

    let scene = null;
    let palette = null;
    let state = emptyState();
    let raf = 0;
    let lastNow = 0;
    let tabVisible = !document.hidden;
    let onScreen = true;
    let disposed = false;

    // ---- tokens -------------------------------------------------------
    function readPalette() {
      const computed = getComputedStyle(canvas);
      const pick = (name, fallback) => computed.getPropertyValue(name).trim() || fallback;
      palette = {
        line: pick('--b10x-color-line', '#898b88'),
        surface: pick('--b10x-color-surface', '#fbf9f2'),
        surfaceMuted: pick('--b10x-color-surface-muted', '#eae7de'),
        heading: pick('--b10x-color-heading', '#08131e'),
        muted: pick('--b10x-color-muted', '#52616a'),
        lime: pick('--b10x-color-lime', '#8ebf3d'),
        cyan: pick('--b10x-color-cyan', '#1686a1'),
        mono: pick('--b10x-font-mono', 'ui-monospace, monospace'),
      };
    }

    // ---- simulation ---------------------------------------------------
    function spawn(edgeIndex, depth) {
      if (state.particles.length >= PARTICLE_LIMIT) return;
      state.particles.push({edge: edgeIndex, t: 0, depth});
    }

    function fireTurn() {
      scene.seeds.forEach((edgeIndex) => spawn(edgeIndex, 0));
      state.flash = 1;
    }

    function step(dt) {
      if (!scene) return;

      // Activations decay whatever else happens, so a lit box fades rather than snapping off.
      for (let i = 0; i < state.boxAct.length; i += 1) state.boxAct[i] *= Math.exp(-dt * 1.5);
      for (let i = 0; i < state.edgeAct.length; i += 1) state.edgeAct[i] *= Math.exp(-dt * 2.1);
      state.flash *= Math.exp(-dt * 3.2);

      // Particles ride their connection; arrival lights the box and cascades onward, to a depth.
      const arrived = [];
      state.particles = state.particles.filter((p) => {
        p.t += dt * EDGE_SPEED;
        state.edgeAct[p.edge] = Math.max(state.edgeAct[p.edge] || 0, 0.85);
        if (p.t < 1) return true;
        arrived.push(p);
        return false;
      });
      arrived.forEach((p) => {
        const edge = scene.edges[p.edge];
        state.boxAct[edge.target] = 1;
        if (p.depth + 1 >= MAX_CASCADE_DEPTH) return;
        scene.outgoing[edge.target].forEach((next) => spawn(next, p.depth + 1));
      });

      if (state.hold > 0) {
        state.hold -= dt;
        state.verdict = Math.min(1, state.verdict + dt * 4);
        if (state.hold <= 0) {
          const keep = scene.boxes.map((_, i) => (state.boxAct[i] || 0) * 0.2);
          state = emptyState();
          state.boxAct = keep;
          state.edgeAct = scene.edges.map(() => 0);
        }
        return;
      }

      const before = state.u;
      state.u += dt / TURN_SECONDS;

      if (before < 1 / 3 && state.u >= 1 / 3 && state.fired !== state.turn) {
        state.fired = state.turn;
        fireTurn();
      }
      if (before < 2 / 3 && state.u >= 2 / 3) {
        state.reached = state.turn >= TURNS_TO_REACH;
        state.verdict = 0;
        if (state.reached) {
          state.flash = 1;
          state.hold = HOLD_SECONDS;
        }
      }
      if (state.u >= 2 / 3) state.verdict = Math.min(1, state.verdict + dt * 4);
      if (state.u >= 1) {
        state.u -= 1;
        state.turn += 1;
        state.verdict = 0;
      }
    }

    // ---- drawing ------------------------------------------------------
    function ringPoint(u) {
      const a = Math.PI + u * Math.PI * 2;
      return {
        x: scene.ring.cx + Math.cos(a) * scene.ring.rx,
        y: scene.ring.cy + Math.sin(a) * scene.ring.ry,
        a,
      };
    }

    /** The token eases between stations rather than gliding at a constant rate. */
    function easedU(u) {
      const segment = Math.floor(u * 3);
      const local = u * 3 - segment;
      return (segment + smooth(local)) / 3;
    }

    function drawRing() {
      const {cx, cy, rx, ry} = scene.ring;
      ctx.save();
      ctx.setLineDash([4, 5]);
      ctx.lineWidth = 1;
      ctx.strokeStyle = alpha(palette.line, 0.85);
      ctx.beginPath();
      ctx.ellipse(cx, cy, rx, ry, 0, 0, Math.PI * 2);
      ctx.stroke();
      ctx.restore();

      // The segment being travelled is lit, so the direction of the loop is never ambiguous.
      const u = easedU(state.u);
      const head = Math.PI + u * Math.PI * 2;
      const tail = head - Math.PI * 0.5;
      const accent = state.reached ? palette.lime : palette.cyan;
      const gradientSteps = 22;
      ctx.lineCap = 'round';
      for (let i = 0; i < gradientSteps; i += 1) {
        const a0 = tail + ((head - tail) * i) / gradientSteps;
        const a1 = tail + ((head - tail) * (i + 1)) / gradientSteps;
        ctx.beginPath();
        ctx.ellipse(cx, cy, rx, ry, 0, a0, a1);
        ctx.lineWidth = 1.4 + (i / gradientSteps) * 1.6;
        ctx.strokeStyle = alpha(accent, 0.06 + (i / gradientSteps) * 0.85);
        ctx.stroke();
      }
    }

    function drawStations() {
      const r = scene.stationR;
      const u = easedU(state.u);
      const accent = state.reached ? palette.lime : palette.cyan;

      scene.stations.forEach((s, index) => {
        // A station glows as the token sits on it.
        const at = index / 3;
        const distance = Math.min(Math.abs(u - at), 1 - Math.abs(u - at));
        const near = clamp(1 - distance / 0.09, 0, 1);
        const isGoal = index === 2;

        if (near > 0) {
          ctx.beginPath();
          ctx.arc(s.x, s.y, r + 5 + near * 5, 0, Math.PI * 2);
          ctx.fillStyle = alpha(accent, 0.16 * near);
          ctx.fill();
        }

        ctx.beginPath();
        ctx.arc(s.x, s.y, r, 0, Math.PI * 2);
        ctx.fillStyle = palette.surface;
        ctx.fill();
        ctx.lineWidth = 1.2;
        ctx.strokeStyle = near > 0.15 ? accent : alpha(palette.line, 0.95);
        ctx.stroke();

        if (index === 1) {
          // The coordinator: a filled core that flares on the turn it fires.
          ctx.beginPath();
          ctx.arc(s.x, s.y, r * (0.38 + state.flash * 0.3), 0, Math.PI * 2);
          ctx.fillStyle = alpha(accent, 0.55 + state.flash * 0.45);
          ctx.fill();
        }
        if (isGoal) {
          // The goal carries its own progress: one notch per turn taken, closed when reached.
          const progress = state.reached ? 1 : (state.turn - 1 + (state.u >= 2 / 3 ? 1 : 0)) / TURNS_TO_REACH;
          ctx.beginPath();
          ctx.arc(s.x, s.y, r + 3.5, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2 * clamp(progress, 0, 1));
          ctx.lineWidth = 2.2;
          ctx.strokeStyle = state.reached ? palette.lime : alpha(palette.cyan, 0.9);
          ctx.stroke();
          if (state.reached) {
            ctx.beginPath();
            ctx.arc(s.x, s.y, r * 0.45, 0, Math.PI * 2);
            ctx.fillStyle = palette.lime;
            ctx.fill();
          }
        }

        ctx.font = `700 ${scene.fontLabel}px ${palette.mono}`;
        ctx.fillStyle = near > 0.15 ? palette.heading : palette.muted;
        const label = s.label.toUpperCase();
        if (s.align === 'right') {
          ctx.textAlign = 'right';
          ctx.textBaseline = 'middle';
          ctx.fillText(label, s.x - r - 8, s.y);
        } else if (s.align === 'top') {
          ctx.textAlign = 'center';
          ctx.textBaseline = 'bottom';
          ctx.fillText(label, s.x, s.y - r - 7);
        } else {
          ctx.textAlign = 'center';
          ctx.textBaseline = 'top';
          ctx.fillText(label, s.x, s.y + r + 7);
        }
      });

      // The token itself, with a short comet behind it.
      for (let i = 6; i >= 0; i -= 1) {
        const p = ringPoint(easedU(clamp(state.u - i * 0.006, 0, 1)));
        ctx.beginPath();
        ctx.arc(p.x, p.y, 3.6 - i * 0.32, 0, Math.PI * 2);
        ctx.fillStyle = alpha(accent, 0.95 - i * 0.13);
        ctx.fill();
      }
    }

    function drawEdges() {
      scene.edges.forEach((edge, index) => {
        const lit = state.edgeAct[index] || 0;
        ctx.beginPath();
        ctx.moveTo(edge.from.x, edge.from.y);
        const midX = (edge.from.x + edge.to.x) / 2;
        ctx.bezierCurveTo(midX, edge.from.y, midX, edge.to.y, edge.to.x, edge.to.y);
        ctx.lineWidth = 1 + lit * 0.6;
        ctx.strokeStyle = lit > 0.02 ? alpha(palette.cyan, 0.25 + lit * 0.6) : alpha(palette.line, 0.55);
        ctx.stroke();
      });
    }

    function drawBoxes() {
      ctx.textAlign = 'left';
      ctx.textBaseline = 'middle';
      scene.boxes.forEach((box, index) => {
        const lit = clamp(state.boxAct[index] || 0, 0, 1);
        roundRect(ctx, box.x, box.y, box.w, box.h, 5);
        ctx.fillStyle = lit > 0.02 ? alpha(palette.lime, 0.1 + lit * 0.22) : palette.surface;
        ctx.fill();
        ctx.lineWidth = 1 + lit * 0.5;
        ctx.strokeStyle = lit > 0.05 ? alpha(palette.lime, 0.55 + lit * 0.45) : alpha(palette.line, 0.9);
        ctx.stroke();

        ctx.font = `700 ${scene.fontBox}px ${palette.mono}`;
        ctx.fillStyle = lit > 0.25 ? palette.heading : palette.muted;
        ctx.fillText(box.name.toUpperCase(), box.x + 8, box.y + box.h / 2);

        // Ports: a box shows only what goes in and what comes out.
        [box.x, box.x + box.w].forEach((px, side) => {
          ctx.beginPath();
          ctx.arc(px, box.y + box.h / 2, 2.4, 0, Math.PI * 2);
          ctx.fillStyle = lit > 0.25 && side === 1 ? palette.lime : alpha(palette.line, 1);
          ctx.fill();
        });
      });
    }

    function drawParticles() {
      state.particles.forEach((p) => {
        const edge = scene.edges[p.edge];
        const t = clamp(p.t, 0, 1);
        const midX = (edge.from.x + edge.to.x) / 2;
        // Position on the same cubic the edge is drawn along.
        const mt = 1 - t;
        const x =
          mt * mt * mt * edge.from.x +
          3 * mt * mt * t * midX +
          3 * mt * t * t * midX +
          t * t * t * edge.to.x;
        const y =
          mt * mt * mt * edge.from.y +
          3 * mt * mt * t * edge.from.y +
          3 * mt * t * t * edge.to.y +
          t * t * t * edge.to.y;
        const fade = 1 - p.depth / (MAX_CASCADE_DEPTH + 1);
        ctx.beginPath();
        ctx.arc(x, y, 2.6 - p.depth * 0.4, 0, Math.PI * 2);
        ctx.fillStyle = alpha(palette.cyan, 0.45 + fade * 0.55);
        ctx.fill();
      });
    }

    function drawReadout() {
      const {pad} = scene;
      ctx.font = `800 ${scene.fontRead}px ${palette.mono}`;
      ctx.textAlign = 'left';
      ctx.textBaseline = 'top';
      ctx.fillStyle = palette.muted;
      const turn = String(Math.min(state.turn, TURNS_TO_REACH)).padStart(2, '0');
      ctx.fillText(`TURN ${turn}  ·  TICK 30s  ·  SERIAL_PER_INSTANCE`, pad, pad * 0.6);

      // The verdict, printed where it is decided: beside the goal.
      if (state.verdict <= 0.01) return;
      const goal = scene.stations[2];
      const text = state.reached ? 'REACHED' : 'NOT YET';
      const accent = state.reached ? palette.lime : palette.cyan;
      ctx.font = `800 ${scene.fontLabel}px ${palette.mono}`;
      const w = ctx.measureText(text).width + 18;
      const h = scene.fontLabel + 12;
      const x = goal.x - w / 2;
      const y = goal.y + scene.stationR + scene.fontLabel + 14;
      ctx.globalAlpha = state.verdict;
      roundRect(ctx, x, y, w, h, 999);
      ctx.fillStyle = alpha(accent, 0.14);
      ctx.fill();
      ctx.lineWidth = 1;
      ctx.strokeStyle = alpha(accent, 0.9);
      ctx.stroke();
      ctx.fillStyle = accent;
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText(text, x + w / 2, y + h / 2 + 0.5);
      ctx.globalAlpha = 1;
    }

    function draw() {
      if (!scene || !palette || scene.width <= 0 || scene.height <= 0) return;
      ctx.clearRect(0, 0, scene.width, scene.height);
      drawEdges();
      drawBoxes();
      drawParticles();
      drawRing();
      drawStations();
      drawReadout();
    }

    // ---- frames -------------------------------------------------------
    function frame(now) {
      raf = 0;
      if (disposed) return;
      const dt = lastNow ? clamp((now - lastNow) / 1000, 0, 1 / 20) : 1 / 60;
      lastNow = now;
      step(dt);
      draw();
      schedule();
    }

    function schedule() {
      if (disposed || raf || !tabVisible || !onScreen || motionQuery.matches) return;
      raf = requestAnimationFrame(frame);
    }

    function stop() {
      if (raf) cancelAnimationFrame(raf);
      raf = 0;
      lastNow = 0;
    }

    /** Reduced motion: the same simulation, advanced to a composed frame, then left alone. */
    function composeStillFrame() {
      state = emptyState();
      for (let i = 0; i < STILL_FRAME_STEPS; i += 1) step(1 / 60);
      draw();
    }

    function resize() {
      const rect = canvas.getBoundingClientRect();
      const width = Math.max(1, Math.round(rect.width));
      const height = Math.max(1, Math.round(rect.height));
      const dpr = clamp(window.devicePixelRatio || 1, 1, 3);
      canvas.width = Math.round(width * dpr);
      canvas.height = Math.round(height * dpr);
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      scene = buildScene(width, height);
      state.boxAct = scene.boxes.map((_, i) => state.boxAct[i] || 0);
      state.edgeAct = scene.edges.map((_, i) => state.edgeAct[i] || 0);
      state.particles = state.particles.filter((p) => p.edge < scene.edges.length);
      if (motionQuery.matches) composeStillFrame();
      else draw();
    }

    // ---- wiring -------------------------------------------------------
    readPalette();
    resize();

    const resizeObserver = new ResizeObserver(() => resize());
    resizeObserver.observe(canvas);

    const intersectionObserver = new IntersectionObserver(
      (entries) => {
        onScreen = entries.some((entry) => entry.isIntersecting);
        if (onScreen) schedule();
        else stop();
      },
      {threshold: 0},
    );
    intersectionObserver.observe(canvas);

    const onVisibility = () => {
      tabVisible = !document.hidden;
      if (tabVisible) schedule();
      else stop();
    };
    document.addEventListener('visibilitychange', onVisibility);

    const onMotionChange = () => {
      stop();
      if (motionQuery.matches) composeStillFrame();
      else schedule();
    };
    motionQuery.addEventListener('change', onMotionChange);

    // A theme switch changes the tokens under us; reread them and repaint.
    const themeObserver = new MutationObserver(() => {
      readPalette();
      if (motionQuery.matches) composeStillFrame();
      else draw();
    });
    themeObserver.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['data-theme'],
    });

    if (motionQuery.matches) composeStillFrame();
    else schedule();

    return () => {
      disposed = true;
      stop();
      resizeObserver.disconnect();
      intersectionObserver.disconnect();
      themeObserver.disconnect();
      motionQuery.removeEventListener('change', onMotionChange);
      document.removeEventListener('visibilitychange', onVisibility);
    };
  }, []);

  return (
    <header className="swarm-hero">
      <PageHeader eyebrow={eyebrow} title={title} description={tagline} actions={actions}>
        {children}
      </PageHeader>
      <figure className="swarm-hero__stage">
        <canvas className="swarm-hero__canvas" ref={canvasRef} role="img" aria-label="The loop, drawn: a token travels from the loop to the coordinator to the goal; each pass of the coordinator fires a turn whose events cascade through the boxes a swarm has drawn, and each arrival at the goal prints a verdict — not yet, until reached." />
        <figcaption className="swarm-hero__legend">
          <span className="swarm-hero__key swarm-hero__key--tick">the tick, every 30 seconds</span>
          <span className="swarm-hero__key swarm-hero__key--cascade">events cascading through bindings</span>
          <span className="swarm-hero__key swarm-hero__key--verdict">the coordinator&rsquo;s verdict</span>
        </figcaption>
      </figure>
    </header>
  );
}
