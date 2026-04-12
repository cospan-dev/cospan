<script lang="ts">
	import type { BranchComparisonResponse } from '$lib/api/schema.js';

	let {
		branch,
		comparison,
		basePath,
	}: {
		branch: { name: string; target: string };
		comparison: BranchComparisonResponse | null;
		basePath: string;
	} = $props();
</script>

<li class="flex items-center gap-3 border-b border-border/50 px-4 py-2.5 last:border-b-0 hover:bg-surface-2/50 transition-colors">
	<!-- Branch name -->
	<a
		href="{basePath}/tree/{branch.name}"
		class="rounded-md bg-focus/10 px-2 py-0.5 font-mono text-xs font-medium text-focus hover:bg-focus/20 transition-colors"
	>
		{branch.name}
	</a>

	<!-- Commit hash link -->
	<a
		href="{basePath}/commit/{branch.target}"
		class="font-mono text-[11px] text-text-muted hover:text-text-secondary transition-colors"
	>
		{branch.target.slice(0, 8)}
	</a>

	<!-- Structural summary -->
	<div class="ml-auto text-[11px]">
		{#if comparison === null}
			<span class="text-text-muted italic">...</span>
		{:else if comparison.breakingCount === 0 && comparison.nonBreakingCount === 0}
			<span class="text-text-muted">no schema changes</span>
		{:else}
			<span class="flex items-center gap-2">
				{#if comparison.addedVertices > 0}
					<span class="text-emerald-400">+{comparison.addedVertices}</span>
				{/if}
				{#if comparison.removedVertices > 0}
					<span class="text-red-400">-{comparison.removedVertices}</span>
				{/if}
				{#if comparison.breakingCount > 0}
					<span class="rounded bg-red-500/15 px-1.5 py-0.5 text-[10px] font-medium text-red-400">
						{comparison.breakingCount} breaking
					</span>
				{:else}
					<span class="rounded bg-emerald-500/15 px-1.5 py-0.5 text-[10px] font-medium text-emerald-400">
						compatible
					</span>
				{/if}
			</span>
		{/if}
	</div>
</li>
