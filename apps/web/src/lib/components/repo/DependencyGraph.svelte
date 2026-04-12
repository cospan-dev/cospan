<script lang="ts">
	import type { DependencyGraphResponse } from '$lib/api/schema.js';
	import { onMount } from 'svelte';

	let {
		graph,
		basePath,
	}: {
		graph: DependencyGraphResponse;
		basePath: string;
	} = $props();

	// ── Force-directed layout ──────────────────────────────────────

	interface NodePos {
		x: number;
		y: number;
		vx: number;
		vy: number;
	}

	const W = 800;
	const H = 500;
	const PAD = 40;

	let positions = $state<NodePos[]>([]);
	let hoveredNode = $state<number | null>(null);
	let ready = $state(false);

	function languageColor(lang: string): string {
		const map: Record<string, string> = {
			typescript: 'oklch(0.65 0.16 230)',
			javascript: 'oklch(0.70 0.16 50)',
			rust: 'oklch(0.60 0.16 25)',
			python: 'oklch(0.65 0.14 210)',
			go: 'oklch(0.65 0.14 170)',
			json: 'oklch(0.65 0.10 45)',
			yaml: 'oklch(0.65 0.10 80)',
			svelte: 'oklch(0.60 0.18 12)',
			'atproto-lexicon': 'oklch(0.65 0.12 195)',
		};
		return map[lang.toLowerCase()] ?? 'oklch(0.60 0.10 260)';
	}

	onMount(() => {
		if (graph.nodes.length === 0) return;

		// Initialize with random positions
		positions = graph.nodes.map(() => ({
			x: PAD + Math.random() * (W - 2 * PAD),
			y: PAD + Math.random() * (H - 2 * PAD),
			vx: 0,
			vy: 0,
		}));

		// Build index maps
		const nodeIndex = new Map<string, number>();
		graph.nodes.forEach((n, i) => nodeIndex.set(n.id, i));

		// Simulation: 80 frames
		let frame = 0;
		const maxFrames = 80;
		const damping = 0.85;
		const repulsion = 3000;
		const attraction = 0.005;
		const idealLength = 120;

		function tick() {
			const ps = [...positions];
			const n = ps.length;

			// Repulsion between all pairs
			for (let i = 0; i < n; i++) {
				for (let j = i + 1; j < n; j++) {
					let dx = ps[i].x - ps[j].x;
					let dy = ps[i].y - ps[j].y;
					const dist = Math.sqrt(dx * dx + dy * dy) || 1;
					const force = repulsion / (dist * dist);
					dx = (dx / dist) * force;
					dy = (dy / dist) * force;
					ps[i].vx += dx;
					ps[i].vy += dy;
					ps[j].vx -= dx;
					ps[j].vy -= dy;
				}
			}

			// Attraction along edges
			for (const edge of graph.edges) {
				const si = nodeIndex.get(edge.src);
				const ti = nodeIndex.get(edge.tgt);
				if (si === undefined || ti === undefined) continue;
				const dx = ps[ti].x - ps[si].x;
				const dy = ps[ti].y - ps[si].y;
				const dist = Math.sqrt(dx * dx + dy * dy) || 1;
				const force = attraction * (dist - idealLength);
				const fx = (dx / dist) * force;
				const fy = (dy / dist) * force;
				ps[si].vx += fx;
				ps[si].vy += fy;
				ps[ti].vx -= fx;
				ps[ti].vy -= fy;
			}

			// Gravity toward center
			for (let i = 0; i < n; i++) {
				ps[i].vx += (W / 2 - ps[i].x) * 0.001;
				ps[i].vy += (H / 2 - ps[i].y) * 0.001;
			}

			// Apply velocities and clamp
			for (let i = 0; i < n; i++) {
				ps[i].vx *= damping;
				ps[i].vy *= damping;
				ps[i].x = Math.max(PAD, Math.min(W - PAD, ps[i].x + ps[i].vx));
				ps[i].y = Math.max(PAD, Math.min(H - PAD, ps[i].y + ps[i].vy));
			}

			positions = ps;
			frame++;
			if (frame < maxFrames) {
				requestAnimationFrame(tick);
			} else {
				ready = true;
			}
		}

		requestAnimationFrame(tick);
	});

	// Build index for quick lookups
	let nodeIndex = $derived(new Map(graph.nodes.map((n, i) => [n.id, i])));
</script>

{#if graph.nodes.length > 0 && positions.length > 0}
	<svg viewBox="0 0 {W} {H}" class="w-full" style="max-height: 500px;">
		<defs>
			<marker id="arrow" viewBox="0 0 10 6" refX="10" refY="3"
				markerWidth="8" markerHeight="6" orient="auto-start-reverse">
				<path d="M 0 0 L 10 3 L 0 6 z" fill="var(--color-line-bright)" />
			</marker>
		</defs>

		<!-- Edges -->
		{#each graph.edges as edge (edge.src + edge.tgt)}
			{@const si = nodeIndex.get(edge.src)}
			{@const ti = nodeIndex.get(edge.tgt)}
			{#if si !== undefined && ti !== undefined}
				{@const sp = positions[si]}
				{@const tp = positions[ti]}
				{@const dx = tp.x - sp.x}
				{@const dy = tp.y - sp.y}
				{@const dist = Math.sqrt(dx * dx + dy * dy) || 1}
				{@const r = Math.max(8, Math.sqrt(graph.nodes[ti].vertexCount) * 2 + 4)}
				<line
					x1={sp.x}
					y1={sp.y}
					x2={tp.x - (dx / dist) * (r + 4)}
					y2={tp.y - (dy / dist) * (r + 4)}
					stroke="var(--color-line)"
					stroke-width="1"
					marker-end="url(#arrow)"
					opacity={ready ? 0.6 : 0.2}
				/>
			{/if}
		{/each}

		<!-- Nodes -->
		{#each graph.nodes as node, i (node.id)}
			{@const p = positions[i]}
			{@const r = Math.max(8, Math.sqrt(node.vertexCount) * 2 + 4)}
			<a href="{basePath}/tree/{node.id}">
				<circle
					cx={p.x}
					cy={p.y}
					{r}
					fill={languageColor(node.language)}
					opacity={hoveredNode === i ? 1 : 0.7}
					stroke={hoveredNode === i ? 'var(--color-ink)' : 'none'}
					stroke-width="1.5"
					class="cursor-pointer transition-opacity"
					onmouseenter={() => (hoveredNode = i)}
					onmouseleave={() => (hoveredNode = null)}
				/>
				<!-- Label -->
				<text
					x={p.x}
					y={p.y + r + 12}
					text-anchor="middle"
					fill="var(--color-caption)"
					font-size="10"
					font-family="var(--font-mono)"
					class="pointer-events-none"
				>
					{node.label}
				</text>
			</a>
		{/each}

		<!-- Tooltip -->
		{#if hoveredNode !== null}
			{@const node = graph.nodes[hoveredNode]}
			{@const p = positions[hoveredNode]}
			<g transform="translate({p.x}, {p.y - Math.max(8, Math.sqrt(node.vertexCount) * 2 + 4) - 8})">
				<rect
					x="-80" y="-30" width="160" height="28" rx="4"
					fill="var(--color-elevated)"
					stroke="var(--color-line)"
					stroke-width="0.5"
				/>
				<text x="0" y="-18" text-anchor="middle" fill="var(--color-ink)" font-size="10" font-family="var(--font-mono)">
					{node.id}
				</text>
				<text x="0" y="-8" text-anchor="middle" fill="var(--color-caption)" font-size="9">
					{node.vertexCount} elements | {node.language}
				</text>
			</g>
		{/if}
	</svg>
{:else}
	<div class="py-12 text-center text-sm text-text-muted">
		No cross-file dependencies detected
	</div>
{/if}
