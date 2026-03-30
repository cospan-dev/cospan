<script lang="ts">
	import PullCard from '$lib/components/pull/PullCard.svelte';
	import RepoTabBar from '$lib/components/repo/RepoTabBar.svelte';
	import Breadcrumb from '$lib/components/shared/Breadcrumb.svelte';
	import BackLink from '$lib/components/shared/BackLink.svelte';
	import EmptyState from '$lib/components/shared/EmptyState.svelte';
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
	<Breadcrumb {crumbs} />

	<div class="mt-3 mb-6">
		<h1 class="text-xl font-semibold text-text-primary">Merge Requests</h1>
	</div>

	<RepoTabBar {basePath} activeTab="pulls" />

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
		<EmptyState icon="merge" message="No {data.filterState} merge requests." />
	{:else}
		<div class="space-y-3">
			{#each data.pulls.items as pull (pull.rkey)}
				<PullCard {pull} {basePath} />
			{/each}
		</div>
	{/if}

	<BackLink href={basePath} />
</section>
