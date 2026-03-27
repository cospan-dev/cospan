<script lang="ts">
	import Breadcrumb from '$lib/components/shared/Breadcrumb.svelte';

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
	<title>Branches - {data.repoName} - Cospan</title>
</svelte:head>

<section>
	<Breadcrumb {crumbs} />

	<div class="mt-4 flex items-center justify-between">
		<h1 class="text-xl font-semibold text-text-primary">Branches</h1>
		<div class="flex items-center gap-2">
			<a
				href="{basePath}/tags"
				class="rounded-md border border-border bg-surface-1 px-3 py-1.5 text-xs text-text-secondary transition-colors hover:border-accent hover:text-text-primary"
			>
				Tags
			</a>
		</div>
	</div>

	{#if data.branches.length === 0}
		<div class="mt-8 flex flex-col items-center gap-3 py-12 text-text-secondary">
			<svg class="h-10 w-10" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
				<path stroke-linecap="round" stroke-linejoin="round" d="M3.75 6A2.25 2.25 0 016 3.75h2.25A2.25 2.25 0 0110.5 6v2.25a2.25 2.25 0 01-2.25 2.25H6a2.25 2.25 0 01-2.25-2.25V6zM3.75 15.75A2.25 2.25 0 016 13.5h2.25a2.25 2.25 0 012.25 2.25V18a2.25 2.25 0 01-2.25 2.25H6A2.25 2.25 0 013.75 18v-2.25zM13.5 6a2.25 2.25 0 012.25-2.25H18A2.25 2.25 0 0120.25 6v2.25A2.25 2.25 0 0118 10.5h-2.25a2.25 2.25 0 01-2.25-2.25V6z" />
			</svg>
			<p class="text-sm">No branches found.</p>
			<p class="text-xs">The node may be unreachable, or this repository has not been initialized.</p>
		</div>
	{:else}
		<div class="mt-4 rounded-lg border border-border bg-surface-1">
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
</section>
