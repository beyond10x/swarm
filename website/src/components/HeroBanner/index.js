import React, {useMemo} from 'react';
import Heading from '@theme/Heading';

// deterministic pseudo-random layout so server and client render match
function mulberry32(seed) {
  return function () {
    seed |= 0;
    seed = (seed + 0x6d2b79f5) | 0;
    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function useSwarmLayout(nodeCount = 18) {
  return useMemo(() => {
    const rand = mulberry32(42);
    const nodes = Array.from({length: nodeCount}, (_, i) => ({
      id: i,
      top: `${rand() * 90}%`,
      left: `${rand() * 96 + 2}%`,
      size: 4 + rand() * 8,
      duration: 6 + rand() * 8,
      delay: -rand() * 10,
      dx1: `${(rand() - 0.5) * 60}px`,
      dy1: `${(rand() - 0.5) * 60}px`,
      dx2: `${(rand() - 0.5) * 60}px`,
      dy2: `${(rand() - 0.5) * 60}px`,
      dx3: `${(rand() - 0.5) * 60}px`,
      dy3: `${(rand() - 0.5) * 60}px`,
    }));

    const lines = Array.from({length: 10}, () => {
      const a = nodes[Math.floor(rand() * nodes.length)];
      const b = nodes[Math.floor(rand() * nodes.length)];
      const ax = parseFloat(a.left);
      const ay = parseFloat(a.top);
      const bx = parseFloat(b.left);
      const by = parseFloat(b.top);
      const length = Math.hypot(bx - ax, by - ay);
      const angle = (Math.atan2(by - ay, bx - ax) * 180) / Math.PI;
      return {
        top: `${ay}%`,
        left: `${ax}%`,
        width: `${length}%`,
        angle,
        duration: 4 + rand() * 6,
        delay: -rand() * 8,
      };
    });

    return {nodes, lines};
  }, [nodeCount]);
}

export default function HeroBanner({title, tagline}) {
  const {nodes, lines} = useSwarmLayout();

  return (
    <header className="hero-banner">
      <div className="hero-banner__field" />
      <div className="hero-banner__swarm" aria-hidden="true">
        {lines.map((l, i) => (
          <span
            key={`line-${i}`}
            className="hero-banner__line"
            style={{
              top: l.top,
              left: l.left,
              width: l.width,
              transform: `rotate(${l.angle}deg)`,
              animationDuration: `${l.duration}s`,
              animationDelay: `${l.delay}s`,
            }}
          />
        ))}
        {nodes.map((n) => (
          <span
            key={n.id}
            className="hero-banner__node"
            style={{
              top: n.top,
              left: n.left,
              width: n.size,
              height: n.size,
              animationDuration: `${n.duration}s`,
              animationDelay: `${n.delay}s`,
              '--dx1': n.dx1,
              '--dy1': n.dy1,
              '--dx2': n.dx2,
              '--dy2': n.dy2,
              '--dx3': n.dx3,
              '--dy3': n.dy3,
            }}
          />
        ))}
      </div>
      <div className="container">
        <Heading as="h1" className="hero-banner__title">
          {title}
        </Heading>
        <p className="hero-banner__tagline">{tagline}</p>
      </div>
    </header>
  );
}
