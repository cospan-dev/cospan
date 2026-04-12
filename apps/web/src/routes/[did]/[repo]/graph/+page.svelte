<script lang="ts">
	import { getContext } from 'svelte';
	import DependencyGraph from '$lib/components/repo/DependencyGraph.svelte';
	import BackLink from '$lib/components/shared/BackLink.svelte';
	import EmptyState from '$lib/components/shared/EmptyState.svelte';

	let { data } = $props();

	let basePath = $derived(`/${data.did}/${data.repoName}`);

	const repoLayout = getContext<any>('repoLayout');
	$effect(() => {
		repoLayout?.setExtraCrumbs([{ label: 'Dependency Graph' }]);
		return () => repoLayout?.setExtraCrumbs([]);
	});
</script>

<svelte:head>
	<title>Dependency Graph · {data.repoName} · Cospan</title>
</svelte:head>

<h1 class="mt-3 mb-6 text-xl font-semibold text-text-primary">Dependency Graph</h1>

{#if data.graph && data.graph.nodes.length > 0}
	<div class="mb-3 flex items-center gap-4 text-xs text-text-muted">
		<span>{data.graph.nodes.length} files</span>
		<span>&middot;</span>
		<span>{data.graph.edges.length} cross-file dependencies</span>
	</div>
	<div class="rounded-lg border border-border bg-surface-1 p-4">
		<DependencyGraph graph={data.graph} {basePath} />
	</div>
{:else}
	<EmptyState
		icon="graph"
		message="Dependency graph unavailable."
		description="The node may be unreachable, no cross-file dependencies were detected, or no files could be parsed."
	/>
{/if}

<BackLink href={basePath} />
