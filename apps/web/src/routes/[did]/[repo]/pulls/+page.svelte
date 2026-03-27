<script lang="ts">
	import PullCard from '$lib/components/pull/PullCard.svelte';
	import Breadcrumb from '$lib/components/shared/Breadcrumb.svelte';
	import KeyboardShortcuts from '$lib/components/shared/KeyboardShortcuts.svelte';

	let { data } = $props();

	let basePath = $derived(`/${data.did}/${data.repo}`);

	let crumbs = $derived([
		{ label: data.did, href: `/${data.did}` },
		{ label: data.repo, href: basePath },
		{ label: 'Merge Requests' }
	]);
</script>

<svelte:head>
	<title>Merge Requests - {data.repo} - Cospan</title>
</svelte:head>

<KeyboardShortcuts {basePath} />

<section>
	<div class="mb-6">
		<Breadcrumb {crumbs} />
		<h1 class="mt-3 text-xl font-semibold text-text-primary">Merge Requests</h1>
	</div>

	<div class="mb-4 flex items-center gap-2">
		<a
			href="{basePath}/pulls?state=open"
			class="rounded-md px-3 py-1.5 text-sm transition-colors {data.filterState === 'open'
				? 'bg-surface-2 text-text-primary font-medium'
				: 'text-text-secondary hover:text-text-primary'}"
		>
			Open
		</a>
		<a
			href="{basePath}/pulls?state=merged"
			class="rounded-md px-3 py-1.5 text-sm transition-colors {data.filterState === 'merged'
				? 'bg-surface-2 text-text-primary font-medium'
				: 'text-text-secondary hover:text-text-primary'}"
		>
			Merged
		</a>
		<a
			href="{basePath}/pulls?state=closed"
			class="rounded-md px-3 py-1.5 text-sm transition-colors {data.filterState === 'closed'
				? 'bg-surface-2 text-text-primary font-medium'
				: 'text-text-secondary hover:text-text-primary'}"
		>
			Closed
		</a>
	</div>

	{#if data.pulls.items.length === 0}
		<div class="flex flex-col items-center gap-4 py-12 text-text-secondary">
			<svg class="h-12 w-12" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
				<path stroke-linecap="round" stroke-linejoin="round" d="M7.5 21L3 16.5m0 0L7.5 12M3 16.5h13.5m0-13.5L21 7.5m0 0L16.5 12M21 7.5H7.5" />
			</svg>
			<p>No {data.filterState} merge requests.</p>
		</div>
	{:else}
		<div class="space-y-3">
			{#each data.pulls.items as pull (pull.rkey)}
				<PullCard {pull} {basePath} />
			{/each}
		</div>
	{/if}

	<div class="mt-6">
		<a
			href={basePath}
			class="inline-flex items-center gap-1.5 text-sm text-accent transition-colors hover:text-accent-hover"
		>
			<svg class="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
				<path stroke-linecap="round" stroke-linejoin="round" d="M10.5 19.5L3 12m0 0l7.5-7.5M3 12h18" />
			</svg>
			Back to repository
		</a>
	</div>
</section>
