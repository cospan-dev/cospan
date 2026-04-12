<script lang="ts">
	import type { BranchComparisonResponse } from '$lib/api/schema.js';
	import type { DependencyEntry } from '$lib/api/search.js';

	let {
		comparison,
		dependents = [],
		loading = false,
	}: {
		comparison: BranchComparisonResponse | null;
		dependents?: DependencyEntry[];
		loading?: boolean;
	} = $props();

	let expanded = $state(false);
	let impactExpanded = $state(false);
</script>

{#if loading}
	<div class="h-16 animate-pulse rounded-lg border border-border bg-surface-0"></div>
{:else if comparison}
	<div
		class="rounded-lg border px-4 py-3 {comparison.compatible
			? 'border-emerald-500/30 bg-emerald-500/5'
			: 'border-red-500/30 bg-red-500/5'}"
	>
		<!-- Verdict row -->
		<div class="flex items-center gap-3">
			{#if comparison.compatible}
				<svg class="h-6 w-6 text-emerald-400" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2.5">
					<path stroke-linecap="round" stroke-linejoin="round" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
				</svg>
				<div>
					<span class="text-lg font-bold text-emerald-400">COMPATIBLE</span>
					<span class="ml-3 text-xs text-text-muted">
						+{comparison.addedVertices} elements &middot;
						{comparison.nonBreakingCount} compatible {comparison.nonBreakingCount === 1 ? 'change' : 'changes'} &middot;
						{comparison.changedFiles.length} {comparison.changedFiles.length === 1 ? 'file' : 'files'}
					</span>
				</div>
			{:else}
				<svg class="h-6 w-6 text-red-400" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2.5">
					<path stroke-linecap="round" stroke-linejoin="round" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
				</svg>
				<div class="flex-1">
					<span class="text-lg font-bold text-red-400">BREAKING CHANGES</span>
					<span class="ml-3 text-xs text-text-muted">
						{comparison.breakingCount} breaking &middot;
						{comparison.nonBreakingCount} compatible &middot;
						{comparison.changedFiles.length} {comparison.changedFiles.length === 1 ? 'file' : 'files'}
					</span>
				</div>
				{#if comparison.breakingChanges.length > 0}
					<button
						type="button"
						class="text-xs text-text-muted hover:text-text-secondary transition-colors"
						onclick={() => (expanded = !expanded)}
					>
						{expanded ? 'hide' : 'show'} details
					</button>
				{/if}
			{/if}
		</div>

		<!-- Expandable breaking change details -->
		{#if expanded && !comparison.compatible && comparison.breakingChanges.length > 0}
			<div class="mt-3 space-y-1 border-t border-red-500/20 pt-3">
				{#each comparison.breakingChanges.slice(0, 10) as change (change.label)}
					<div class="flex items-start gap-2 text-sm">
						<span class="mt-0.5 shrink-0 text-red-400">&#x26A0;</span>
						<span class="text-text-primary">{change.label}</span>
						<span class="ml-auto shrink-0 rounded bg-surface-2 px-1.5 py-0.5 text-[10px] text-text-muted">
							{change.kind}
						</span>
					</div>
				{/each}
				{#if comparison.breakingChanges.length > 10}
					<div class="text-xs text-text-muted">
						... and {comparison.breakingChanges.length - 10} more
					</div>
				{/if}
			</div>
		{/if}

		<!-- Downstream impact (when breaking changes exist and dependents are known) -->
		{#if !comparison.compatible && dependents.length > 0}
			<div class="mt-3 border-t border-red-500/20 pt-3">
				<button
					type="button"
					class="flex w-full items-center gap-2 text-xs font-medium text-amber-400 hover:text-amber-300 transition-colors"
					onclick={() => (impactExpanded = !impactExpanded)}
				>
					<svg class="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
						<path stroke-linecap="round" stroke-linejoin="round" d="M13 10V3L4 14h7v7l9-11h-7z" />
					</svg>
					Downstream impact: {dependents.length} dependent {dependents.length === 1 ? 'repo' : 'repos'} may be affected
					<svg
						class="ml-auto h-3 w-3 transition-transform {impactExpanded ? 'rotate-90' : ''}"
						fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"
					>
						<path stroke-linecap="round" stroke-linejoin="round" d="M9 5l7 7-7 7" />
					</svg>
				</button>
				{#if impactExpanded}
					<div class="mt-2 space-y-1">
						{#each dependents as dep (dep.did + '/' + dep.repo)}
							<a
								href="/{dep.did}/{dep.repo}"
								class="flex items-center gap-2 rounded-md px-2 py-1 text-sm text-text-secondary hover:bg-surface-2 transition-colors"
							>
								<span class="font-mono text-xs text-accent">{dep.repo}</span>
								<span class="text-[10px] text-text-muted">{dep.did}</span>
							</a>
						{/each}
					</div>
				{/if}
			</div>
		{/if}
	</div>
{/if}
