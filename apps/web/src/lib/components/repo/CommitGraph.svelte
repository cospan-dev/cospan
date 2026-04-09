<script lang="ts">
	import { onMount } from 'svelte';
	import type { Commit, CommitGraphLayout } from '$lib/api/vcs.js';
	import { layoutCommitGraph } from '$lib/api/vcs.js';
	import { resolveHandle } from '$lib/api/handle.js';

	let {
		commits,
		basePath = '',
		commitUrlBase = ''
	}: {
		commits: Commit[];
		basePath?: string;
		commitUrlBase?: string;
	} = $props();

	// Resolve committer emails → DIDs → handles. For now we just show
	// the author.name from the git commit, which for Cospan-created
	// commits is the authenticated DID.
	let handles = $state<Record<string, string>>({});

	onMount(async () => {
		const candidates = new Set<string>();
		for (const c of commits) {
			if (c.author?.name?.startsWith('did:')) candidates.add(c.author.name);
			if (c.committer?.name?.startsWith('did:')) candidates.add(c.committer.name);
		}
		const resolved: Record<string, string> = {};
		await Promise.allSettled(
			Array.from(candidates).map(async (did) => {
				resolved[did] = await resolveHandle(did);
			})
		);
		handles = resolved;
	});

	function displayAuthor(c: Commit): string {
		const name = c.author?.name ?? '';
		if (name.startsWith('did:')) {
			return handles[name] || name.slice(8, 18) + '…';
		}
		return name || c.author?.email || 'unknown';
	}

	function formatTime(unixSeconds: number): string {
		if (!unixSeconds) return '';
		const d = new Date(unixSeconds * 1000);
		return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' });
	}

	function truncate(oid: string): string {
		return oid.slice(0, 8);
	}

	// Build layout from the commits list.
	let layout: CommitGraphLayout = $derived(layoutCommitGraph(commits));

	// Lane colours — same palette as the design system's accent family.
	const LANE_COLORS = [
		'#6366f1', // indigo
		'#10b981', // emerald
		'#f59e0b', // amber
		'#ec4899', // pink
		'#8b5cf6', // violet
		'#14b8a6', // teal
		'#f97316', // orange
		'#06b6d4', // cyan
	];

	function laneColor(lane: number): string {
		return LANE_COLORS[lane % LANE_COLORS.length];
	}

	// Geometry
	const ROW_HEIGHT = 36;
	const LANE_WIDTH = 18;
	const NODE_RADIUS = 5;
	const LEFT_PADDING = 14;

	function laneX(lane: number): number {
		return LEFT_PADDING + lane * LANE_WIDTH;
	}

	function rowY(row: number): number {
		return ROW_HEIGHT / 2 + row * ROW_HEIGHT;
	}

	let graphWidth = $derived(Math.max(1, layout.laneCount) * LANE_WIDTH + LEFT_PADDING * 2);
	let graphHeight = $derived(Math.max(1, commits.length) * ROW_HEIGHT);

	// For each edge, build an SVG path. If fromLane == toLane, it's a
	// straight vertical line. Otherwise, we route it as an S-curve:
	// vertical from the child down to the midpoint, then curve over to
	// the parent's lane, then vertical down to the parent.
	function edgePath(fromRow: number, toRow: number, fromLane: number, toLane: number): string {
		const x1 = laneX(fromLane);
		const y1 = rowY(fromRow) + NODE_RADIUS;
		const x2 = laneX(toLane);
		const y2 = rowY(toRow) - NODE_RADIUS;
		if (fromLane === toLane) {
			return `M ${x1} ${y1} L ${x2} ${y2}`;
		}
		// S-curve via a cubic Bezier. Control points at the lane midpoints
		// create a smooth diagonal when lanes shift by one.
		const midY = (y1 + y2) / 2;
		return `M ${x1} ${y1} C ${x1} ${midY}, ${x2} ${midY}, ${x2} ${y2}`;
	}

	function commitHref(oid: string): string {
		if (commitUrlBase) return `${commitUrlBase}/${oid}`;
		return `${basePath}/commit/${oid}`;
	}
</script>

{#if commits.length === 0}
	<p class="py-8 text-center text-sm text-text-secondary">No commits yet.</p>
{:else}
	<div class="flex">
		<!-- SVG graph column -->
		<svg
			width={graphWidth}
			height={graphHeight}
			class="shrink-0"
			role="img"
			aria-label="Commit graph"
		>
			<!-- Edges first so nodes sit on top -->
			{#each layout.edges as edge (edge.fromRow + '_' + edge.toRow + '_' + edge.fromLane + '_' + edge.toLane)}
				<path
					d={edgePath(edge.fromRow, edge.toRow, edge.fromLane, edge.toLane)}
					stroke={laneColor(edge.fromLane === edge.toLane ? edge.fromLane : edge.toLane)}
					stroke-width="2"
					fill="none"
					stroke-linecap="round"
					opacity="0.85"
				/>
			{/each}

			<!-- Nodes -->
			{#each layout.nodes as node (node.commit.oid)}
				<circle
					cx={laneX(node.lane)}
					cy={rowY(node.row)}
					r={NODE_RADIUS}
					fill={laneColor(node.lane)}
					stroke="var(--color-surface-1, #0a0a0a)"
					stroke-width="2"
				>
					<title>{node.commit.oid}</title>
				</circle>
				{#if node.commit.parents.length >= 2}
					<!-- Merge commits get a hollow ring -->
					<circle
						cx={laneX(node.lane)}
						cy={rowY(node.row)}
						r={NODE_RADIUS - 1.5}
						fill="var(--color-surface-1, #0a0a0a)"
					/>
				{/if}
			{/each}
		</svg>

		<!-- Commit rows, aligned 1:1 with graph rows -->
		<ul class="flex-1 min-w-0">
			{#each layout.nodes as node (node.commit.oid)}
				<li style="height: {ROW_HEIGHT}px;" class="flex items-center">
					<a
						href={commitHref(node.commit.oid)}
						class="group flex min-w-0 flex-1 items-center gap-3 rounded px-2 py-1 transition-colors hover:bg-surface-2"
					>
						<span class="min-w-0 flex-1 truncate text-sm text-text-primary">
							{node.commit.summary || '(no message)'}
						</span>
						<code class="shrink-0 font-mono text-[11px] text-text-muted group-hover:text-accent">
							{truncate(node.commit.oid)}
						</code>
						<span class="shrink-0 text-[11px] text-text-muted hidden sm:inline">
							{displayAuthor(node.commit)}
						</span>
						<time class="shrink-0 text-[11px] text-text-muted hidden md:inline">
							{formatTime(node.commit.timestamp)}
						</time>
					</a>
				</li>
			{/each}
		</ul>
	</div>
{/if}
