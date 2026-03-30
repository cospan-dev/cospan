<script lang="ts">
	import { getContext } from 'svelte';
	import PullCard from '$lib/components/pull/PullCard.svelte';
	import BackLink from '$lib/components/shared/BackLink.svelte';
	import EmptyState from '$lib/components/shared/EmptyState.svelte';

	let { data } = $props();

	let basePath = $derived(`/${data.did}/${data.repo}`);

	const repoLayout = getContext<any>('repoLayout');
	$effect(() => {
		repoLayout?.setExtraCrumbs([{ label: 'Merge Requests' }]);
		return () => repoLayout?.setExtraCrumbs([]);
	});
</script>

<svelte:head>
	<title>Merge Requests · {data.repo} · Cospan</title>
</svelte:head>

<div class="mt-3 mb-6">
	<h1 class="text-xl font-semibold text-text-primary">Merge Requests</h1>
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
	<EmptyState icon="merge" message="No {data.filterState} merge requests." />
{:else}
	<div class="space-y-3">
		{#each data.pulls.items as pull (pull.rkey)}
			<PullCard {pull} {basePath} />
		{/each}
	</div>
{/if}

<BackLink href={basePath} />
