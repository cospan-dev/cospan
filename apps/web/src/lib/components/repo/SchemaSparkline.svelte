<script lang="ts">
	import type { CommitSchemaStat } from '$lib/api/schema.js';

	let {
		stats,
		width = 280,
		height = 48,
	}: {
		stats: CommitSchemaStat[];
		width?: number;
		height?: number;
	} = $props();

	let hoveredIndex = $state<number | null>(null);

	// Reverse to oldest-first for left-to-right rendering
	let ordered = $derived([...stats].reverse());

	let values = $derived(ordered.map((c) => c.totalVertexCount));
	let minVal = $derived(Math.min(...values));
	let maxVal = $derived(Math.max(...values));
	let range = $derived(maxVal - minVal || 1);

	function x(i: number): number {
		if (ordered.length <= 1) return width / 2;
		return (i / (ordered.length - 1)) * (width - 8) + 4;
	}

	function y(val: number): number {
		return height - 6 - ((val - minVal) / range) * (height - 12);
	}

	let pathD = $derived(
		ordered
			.map((c, i) => `${i === 0 ? 'M' : 'L'}${x(i).toFixed(1)},${y(c.totalVertexCount).toFixed(1)}`)
			.join(' ')
	);
</script>

{#if ordered.length > 1}
	<div class="relative">
		<svg {width} {height} class="overflow-visible">
			<!-- Line -->
			<path
				d={pathD}
				fill="none"
				stroke="var(--color-focus)"
				stroke-width="1.5"
				stroke-linecap="round"
				stroke-linejoin="round"
			/>

			<!-- Dots at significant commits -->
			{#each ordered as commit, i (commit.oid)}
				{#if commit.breakingChangeCount > 0}
					<!-- Breaking change: red dot -->
					<circle
						cx={x(i)}
						cy={y(commit.totalVertexCount)}
						r="3"
						fill="var(--color-err)"
						class="cursor-pointer"
						onmouseenter={() => (hoveredIndex = i)}
						onmouseleave={() => (hoveredIndex = null)}
					/>
				{:else if commit.nonBreakingChangeCount > 0}
					<!-- Compatible change: green dot -->
					<circle
						cx={x(i)}
						cy={y(commit.totalVertexCount)}
						r="2"
						fill="var(--color-ok)"
						class="cursor-pointer"
						onmouseenter={() => (hoveredIndex = i)}
						onmouseleave={() => (hoveredIndex = null)}
					/>
				{:else}
					<!-- Invisible hover target -->
					<circle
						cx={x(i)}
						cy={y(commit.totalVertexCount)}
						r="4"
						fill="transparent"
						class="cursor-pointer"
						onmouseenter={() => (hoveredIndex = i)}
						onmouseleave={() => (hoveredIndex = null)}
					/>
				{/if}
			{/each}
		</svg>

		<!-- Tooltip -->
		{#if hoveredIndex !== null && ordered[hoveredIndex]}
			{@const c = ordered[hoveredIndex]}
			<div
				class="pointer-events-none absolute z-50 rounded-md border border-border bg-elevated px-2.5 py-1.5 text-[11px] shadow-lg"
				style="left: {x(hoveredIndex)}px; top: {y(c.totalVertexCount) - 40}px; transform: translateX(-50%);"
			>
				<div class="font-medium text-text-primary">{c.summary}</div>
				<div class="mt-0.5 text-text-muted">
					{c.totalVertexCount} elements
					{#if c.breakingChangeCount > 0}
						<span class="text-red-400">&middot; {c.breakingChangeCount} breaking</span>
					{/if}
					{#if c.nonBreakingChangeCount > 0}
						<span class="text-emerald-400">&middot; {c.nonBreakingChangeCount} compatible</span>
					{/if}
				</div>
			</div>
		{/if}
	</div>
{/if}
