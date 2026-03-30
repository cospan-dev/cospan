<script lang="ts">
	import RepoTabBar from '$lib/components/repo/RepoTabBar.svelte';
	import Breadcrumb from '$lib/components/shared/Breadcrumb.svelte';
	import BackLink from '$lib/components/shared/BackLink.svelte';
	import EmptyState from '$lib/components/shared/EmptyState.svelte';

	let { data } = $props();

	let basePath = $derived(`/${data.did}/${data.repoName}`);

	let crumbs = $derived([
		{ label: data.did, href: `/${data.did}` },
		{ label: data.repoName, href: basePath },
		{ label: 'Branches' }
	]);

	function truncateHash(hash: string): string {
		return hash.slice(0, 10);
	}
</script>

<svelte:head>
	<title>Branches · {data.repoName} · Cospan</title>
</svelte:head>

<section>
	<Breadcrumb {crumbs} />

	<h1 class="mt-3 mb-6 text-xl font-semibold text-text-primary">Branches</h1>

	<RepoTabBar {basePath} activeTab="branches" />

	{#if data.branches.length === 0}
		<EmptyState
			icon="branch"
			message="No branches found."
			description="The node may be unreachable, or this repository has not been initialized."
		/>
	{:else}
		<div class="rounded-lg border border-border bg-surface-1">
			<ul class="divide-y divide-border">
				{#each data.branches as branch (branch.name)}
					<li class="flex items-center justify-between gap-4 px-4 py-3">
						<div class="flex items-center gap-3 min-w-0">
							<span class="rounded bg-surface-2 px-2 py-0.5 font-mono text-xs text-accent">
								{branch.name}
							</span>
						</div>
						<div class="flex items-center gap-3 shrink-0">
							<a
								href="{basePath}/commit/{branch.target}"
								class="font-mono text-xs text-text-secondary transition-colors hover:text-accent"
								title={branch.target}
							>
								{truncateHash(branch.target)}
							</a>
						</div>
					</li>
				{/each}
			</ul>
		</div>
	{/if}

	<BackLink href={basePath} />
</section>
