<script lang="ts">
	import type { Pull } from '$lib/api/pull.js';
	import StateBadge from '$lib/components/shared/StateBadge.svelte';
	import { timeAgo } from '$lib/utils/time.js';

	let { pull, basePath }: { pull: Pull; basePath: string } = $props();

	let sourceLabel = $derived(pull.sourceRef.replace('refs/heads/', ''));
	let targetLabel = $derived(pull.targetRef.replace('refs/heads/', ''));
</script>

<a
	href="{basePath}/pulls/{pull.rkey}"
	class="block rounded-lg border border-border bg-surface-1 p-4 transition-all hover:border-border-hover"
>
	<div class="flex items-start gap-3">
		<div class="mt-0.5">
			<StateBadge state={pull.state} />
		</div>
		<div class="min-w-0 flex-1">
			<h3 class="font-semibold text-text-primary">{pull.title}</h3>

			<div class="mt-1.5 flex items-center gap-2">
				<span class="rounded bg-accent/10 px-1.5 py-0.5 font-mono text-xs text-accent">
					{sourceLabel}
				</span>
				<svg class="h-3 w-3 text-text-muted" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
					<path stroke-linecap="round" stroke-linejoin="round" d="M13 7l5 5m0 0l-5 5m5-5H6" />
				</svg>
				<span class="rounded bg-surface-2 px-1.5 py-0.5 font-mono text-xs text-text-muted">
					{targetLabel}
				</span>
			</div>

			<div class="mt-1.5 flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-text-muted">
				<span>#{pull.rkey}</span>
				{#if pull.creatorHandle}
					<span>opened by {pull.creatorHandle}</span>
				{/if}
				{#if pull.commentCount > 0}
					<span class="flex items-center gap-1">
						<svg class="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
							<path stroke-linecap="round" stroke-linejoin="round" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
						</svg>
						{pull.commentCount}
					</span>
				{/if}
				<span>{timeAgo(pull.createdAt)}</span>
			</div>

			{#if pull.mergePreview}
				<div class="mt-2 flex items-center gap-3 text-xs">
					{#if pull.mergePreview.breakingChangeCount > 0}
						<span class="text-danger font-medium">
							{pull.mergePreview.breakingChangeCount} breaking
						</span>
					{/if}
					{#if pull.mergePreview.conflictCount > 0}
						<span class="text-warning font-medium">
							{pull.mergePreview.conflictCount} conflicts
						</span>
					{/if}
					{#if pull.mergePreview.lensQuality !== null}
						<span class="text-text-muted">
							lens {(pull.mergePreview.lensQuality * 100).toFixed(0)}%
						</span>
					{/if}
					{#if pull.mergePreview.autoMergeEligible}
						<span class="text-success">auto-merge eligible</span>
					{/if}
				</div>
			{/if}
		</div>
	</div>
</a>
