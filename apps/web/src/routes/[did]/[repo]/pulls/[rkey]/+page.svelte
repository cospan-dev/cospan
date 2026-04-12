<script lang="ts">
	import { getContext } from 'svelte';
	import StateBadge from '$lib/components/shared/StateBadge.svelte';
	import CompatibilityBadge from '$lib/components/repo/CompatibilityBadge.svelte';
	import BackLink from '$lib/components/shared/BackLink.svelte';
	import { timeAgo } from '$lib/utils/time.js';

	let { data } = $props();

	let basePath = $derived(`/${data.did}/${data.repo}`);
	let sourceLabel = $derived(data.pull.sourceRef.replace('refs/heads/', ''));
	let targetLabel = $derived(data.pull.targetRef.replace('refs/heads/', ''));

	const repoLayout = getContext<any>('repoLayout');
	$effect(() => {
		repoLayout?.setExtraCrumbs([
			{ label: 'Merge Requests', href: `${basePath}/pulls` },
			{ label: `#${data.pull.rkey}` }
		]);
		return () => repoLayout?.setExtraCrumbs([]);
	});
</script>

<svelte:head>
	<title>{data.pull.title}· Merge Requests · {data.repo} · Cospan</title>
</svelte:head>

<div class="mt-3 mb-6">
	<div class="flex flex-col gap-2 sm:flex-row sm:items-start sm:gap-3">
		<h1 class="text-2xl font-semibold text-text-primary">{data.pull.title}</h1>
		<div class="shrink-0">
			<StateBadge state={data.pull.state} />
		</div>
	</div>

	<div class="mt-2 flex flex-wrap items-center gap-2 text-sm text-text-secondary">
		<span>
			{data.pull.did} wants to merge
		</span>
		<span class="rounded bg-surface-2 px-1.5 py-0.5 font-mono text-xs text-accent">
			{sourceLabel}
		</span>
		<svg class="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
			<path stroke-linecap="round" stroke-linejoin="round" d="M13 7l5 5m0 0l-5 5m5-5H6" />
		</svg>
		<span class="rounded bg-surface-2 px-1.5 py-0.5 font-mono text-xs text-text-secondary">
			{targetLabel}
		</span>
	</div>
</div>

<!-- Structural compatibility verdict -->
<div class="mb-4">
	<CompatibilityBadge comparison={data.branchComparison} />
</div>

{#if data.pull.body}
	<div class="mb-6 rounded-lg border border-border bg-surface-1">
		<div class="flex items-center gap-2 border-b border-border px-4 py-2">
			<span class="text-sm font-medium text-text-primary">
				{data.pull.did}
			</span>
			<span class="text-xs text-text-secondary">
				opened {timeAgo(data.pull.createdAt)}
			</span>
		</div>
		<div class="px-4 py-3 text-sm text-text-primary whitespace-pre-wrap">
			{data.pull.body}
		</div>
	</div>
{/if}

<div>
	<h2 class="mb-4 text-sm font-medium text-text-primary">
		Comments ({data.comments.items.length})
	</h2>

	{#if data.comments.items.length === 0}
		<p class="py-8 text-center text-sm text-text-secondary">No comments yet.</p>
	{:else}
		<div class="space-y-4">
			{#each data.comments.items as comment (comment.rkey)}
				<div class="rounded-lg border border-border bg-surface-1">
					<div class="flex flex-wrap items-center gap-2 border-b border-border px-4 py-2">
						<span class="text-sm font-medium text-text-primary">
							{comment.did}
						</span>
						{#if comment.reviewDecision}
							{#if comment.reviewDecision === 'approve'}
								<span class="rounded-full bg-compatible/15 px-2 py-0.5 text-xs font-medium text-compatible">
									Approved
								</span>
							{:else if comment.reviewDecision === 'request_changes'}
								<span class="rounded-full bg-breaking/15 px-2 py-0.5 text-xs font-medium text-breaking">
									Changes requested
								</span>
							{/if}
						{/if}
						<span class="text-xs text-text-secondary">
							{timeAgo(comment.createdAt)}
						</span>
					</div>
					<div class="px-4 py-3 text-sm text-text-primary whitespace-pre-wrap">
						{comment.body}
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>

<BackLink href="{basePath}/pulls" label="Back to merge requests" />
