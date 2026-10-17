<script lang="ts">
  const W = 900;
  const H = 1000;
  const BANDS = 24;

  let seed = 11;
  const rand = () => {
    seed = (seed * 9301 + 49297) % 233280;
    return seed / 233280;
  };

  type Pt = [number, number];

  function boundary(i: number): Pt[] {
    const base = -60 + (i * (H + 120)) / BANDS;
    const amp = 12 + rand() * 22;
    const phase = rand() * Math.PI * 2;
    const freq = 0.6 + rand() * 0.9;
    const pts: Pt[] = [];
    for (let x = -700; x <= W + 300; x += 60) {
      const t = x / W;
      const y =
        base +
        amp * Math.sin(t * Math.PI * 2 * freq + phase) +
        amp * 0.45 * Math.sin(t * Math.PI * 2 * freq * 2.7 + phase * 1.9) +
        i * 4 * Math.sin(t * 2.2);
      pts.push([x, y]);
    }
    return pts;
  }

  function smooth(pts: Pt[], reverse = false): string {
    const p = reverse ? [...pts].reverse() : pts;
    let d = '';
    for (let i = 0; i < p.length - 1; i++) {
      const p0 = p[Math.max(0, i - 1)];
      const p1 = p[i];
      const p2 = p[i + 1];
      const p3 = p[Math.min(p.length - 1, i + 2)];
      const c1: Pt = [p1[0] + (p2[0] - p0[0]) / 6, p1[1] + (p2[1] - p0[1]) / 6];
      const c2: Pt = [p2[0] - (p3[0] - p1[0]) / 6, p2[1] - (p3[1] - p1[1]) / 6];
      d += `C${c1[0].toFixed(1)},${c1[1].toFixed(1)} ${c2[0].toFixed(1)},${c2[1].toFixed(1)} ${p2[0].toFixed(1)},${p2[1].toFixed(1)}`;
    }
    return d;
  }

  const bounds: Pt[][] = [];
  for (let i = 0; i <= BANDS; i++) bounds.push(boundary(i));

  const bands = bounds.slice(0, -1).map((top, i) => {
    const bottom = bounds[i + 1];
    const start = top[0];
    const d = `M${start[0]},${start[1]}${smooth(top)}L${bottom[bottom.length - 1][0]},${bottom[bottom.length - 1][1]}${smooth(bottom, true)}Z`;
    const tones = [0.03, 0.06, 0.02, 0.05, 0.04, 0.08];
    return { d, o: tones[i % tones.length], accent: i === 5 };
  });
</script>

<svg class="strata" viewBox="0 0 {W} {H}" preserveAspectRatio="xMaxYMin slice" aria-hidden="true">
  <defs>
    <radialGradient id="strata-fade" cx="0.82" cy="0.4" r="0.78">
      <stop offset="0" stop-color="#fff" stop-opacity="1" />
      <stop offset="0.55" stop-color="#fff" stop-opacity="0.75" />
      <stop offset="1" stop-color="#fff" stop-opacity="0" />
    </radialGradient>
    <mask id="strata-mask" maskUnits="userSpaceOnUse" x="-400" y="-400" width={W + 800} height={H + 800}>
      <rect x="-400" y="-400" width={W + 800} height={H + 800} fill="url(#strata-fade)" />
    </mask>
  </defs>
  <g mask="url(#strata-mask)">
    <g transform="rotate(-7 {W} 0)">
      {#each bands as b, i (i)}
        <path d={b.d} class:accent={b.accent} style:fill-opacity={b.accent ? null : b.o} />
      {/each}
    </g>
  </g>
</svg>

<style>
  .strata {
    width: 100%;
    height: 100%;
    display: block;
  }

  path {
    fill: var(--ink);
    stroke: var(--ink);
    stroke-opacity: 0.08;
    stroke-width: 1;
  }

  path.accent {
    fill: var(--mark);
    fill-opacity: var(--strata-accent-opacity, 0.9);
    mix-blend-mode: var(--strata-accent-blend, multiply);
    stroke: none;
  }
</style>
