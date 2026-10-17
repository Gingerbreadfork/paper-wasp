<script lang="ts">
  let { size = 24, compact = false, class: cls = '' }: { size?: number; compact?: boolean; class?: string } = $props();
  const uid = $props.id();
  const sw = $derived(compact ? 9 : 5);
  const aw = $derived(compact ? 8 : 4);
  const pr = $derived(compact ? 7 : 4.5);
  const band = $derived(compact ? 10 : 7);
</script>

<svg class={cls} width={size} height={size} viewBox="0 0 256 256" aria-hidden="true">
  <defs>
    <clipPath id="abd-{uid}">
      <polygon points="104,146 152,146 158,160 152,204 128,242 104,204 98,160" />
    </clipPath>
  </defs>
  <g fill="none" stroke="currentColor" stroke-width={sw} stroke-linecap="round" stroke-linejoin="round">
    <path d="M100 90 L72 76 L50 80" />
    <path d="M156 90 L184 76 L206 80" />
    <path d="M100 108 L62 108 L44 118" />
    <path d="M156 108 L194 108 L212 118" />
    <path d="M100 126 L74 146 L64 172" />
    <path d="M156 126 L182 146 L192 172" />
    <path d="M118 42 L106 22 L82 16" stroke-width={aw} />
    <path d="M138 42 L150 22 L174 16" stroke-width={aw} />
  </g>
  <g fill="currentColor">
    <circle cx="50" cy="80" r={pr} />
    <circle cx="206" cy="80" r={pr} />
    <circle cx="44" cy="118" r={pr} />
    <circle cx="212" cy="118" r={pr} />
    <circle cx="64" cy="172" r={pr} />
    <circle cx="192" cy="172" r={pr} />
    <circle cx="82" cy="16" r={pr} />
    <circle cx="174" cy="16" r={pr} />
    <rect x="108" y="40" width="40" height="32" rx="8" />
    <rect x="98" y="76" width="60" height="58" rx="10" />
    <rect x="122" y="132" width="12" height="14" />
    <polygon points="104,146 152,146 158,160 152,204 128,242 104,204 98,160" />
  </g>
  <g clip-path="url(#abd-{uid})" fill="var(--mark, #f2e04a)">
    <rect x="90" y="166" width="76" height={band} />
    <rect x="90" y="190" width="76" height={band} />
    <rect x="90" y="214" width="76" height={band - 1} />
  </g>
  <circle cx="128" cy="56" r={compact ? 10 : 8.5} fill="var(--mark, #f2e04a)" />
  <circle cx="129" cy="56" r={compact ? 3.5 : 3} fill="currentColor" />
</svg>
