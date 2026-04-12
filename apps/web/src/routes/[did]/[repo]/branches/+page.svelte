<script lang="ts">
	import { getContext } from 'svelte';
	import BackLink from '$lib/components/shared/BackLink.svelte';
	import EmptyState from '$lib/components/shared/EmptyState.svelte';
	import BranchSchemaRow from '$lib/components/repo/BranchSchemaRow.svelte';

	let { data } = $props();

	let basePath = $derived(`/${data.did}/${data.repoName}`);

	const repoLayout = getContext<any>('repoLayout');
	$effect(() => {
		repoLayout?.setExtraCrumbs([{ label: 'Branches' }]);
		return () => repoLayout?.setExtraCrumbs([]);
	});
</script>

<svelte:head>
	<title>Branches · {data.repoName} · Cospan</title>
</svelte:head>

<h1 class="mt-3 mb-6 text-xl font-semibold text-text-primary">Branches</h1>

{#if data.branches.length === 0}
	<EmptyState
		icon="branch"
		message="No branches found."
		description="The node may be unreachable, or this repository has not been initialized."
	/>
{:else}
	<div class="rounded-lg border border-border bg-surface-1">
		<ul>
			{#each data.branches as branch (branch.name)}
				<BranchSchemaRow
					{branch}
					comparison={data.comparisons[branch.name] ?? null}
					{basePath}
				/>
			{/each}
		</ul>
	</div>
{/if}

<BackLink href={basePath} />
