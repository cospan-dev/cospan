<script lang="ts">
	import RepoCard from '$lib/components/repo/RepoCard.svelte';
	import EmptyState from '$lib/components/shared/EmptyState.svelte';

	let { data } = $props();
</script>

<svelte:head>
	<title>{data.profile?.displayName ?? data.profile?.handle ?? data.did} · Cospan</title>
</svelte:head>

<section>
	<!-- Repositories tab content (default) -->
	{#if data.repos.items.length === 0}
		<EmptyState icon="folder" message="No repositories yet." />
	{:else}
		<div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
			{#each data.repos.items as repo (repo.did + '/' + repo.name)}
				<RepoCard {repo} />
			{/each}
		</div>
	{/if}
</section>
