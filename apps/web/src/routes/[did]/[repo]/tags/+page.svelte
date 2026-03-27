<script lang="ts">
	import Breadcrumb from '$lib/components/shared/Breadcrumb.svelte';

	let { data } = $props();

	let basePath = $derived(`/${data.did}/${data.repoName}`);

	let crumbs = $derived([
		{ label: data.did, href: `/${data.did}` },
		{ label: data.repoName, href: basePath },
		{ label: 'Tags' }
	]);

	function truncateHash(hash: string): string {
		return hash.slice(0, 10);
	}
</script>

<svelte:head>
	<title>Tags - {data.repoName} - Cospan</title>
</svelte:head>

<section>
	<Breadcrumb {crumbs} />

	<div class="mt-4 flex items-center justify-between">
		<h1 class="text-xl font-semibold text-text-primary">Tags</h1>
		<div class="flex items-center gap-2">
			<a
				href="{basePath}/branches"
				class="rounded-md border border-border bg-surface-1 px-3 py-1.5 text-xs text-text-secondary transition-colors hover:border-accent hover:text-text-primary"
			>
				Branches
			</a>
		</div>
	</div>

	{#if data.tags.length === 0}
		<div class="mt-8 flex flex-col items-center gap-3 py-12 text-text-secondary">
			<svg class="h-10 w-10" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
				<path stroke-linecap="round" stroke-linejoin="round" d="M9.568 3H5.25A2.25 2.25 0 003 5.25v4.318c0 .597.237 1.17.659 1.591l9.581 9.581c.699.699 1.78.872 2.607.33a18.095 18.095 0 005.223-5.223c.542-.827.369-1.908-.33-2.607L11.16 3.66A2.25 2.25 0 009.568 3z" />
				<path stroke-linecap="round" stroke-linejoin="round" d="M6 6h.008v.008H6V6z" />
			</svg>
			<p class="text-sm">No tags found.</p>
			<p class="text-xs">Tags are created when a release is published.</p>
		</div>
	{:else}
		<div class="mt-4 rounded-lg border border-border bg-surface-1">
			<ul class="divide-y divide-border">
				{#each data.tags as tag (tag.name)}
					<li class="flex items-center justify-between gap-4 px-4 py-3">
						<div class="flex items-center gap-3 min-w-0">
							<svg class="h-4 w-4 shrink-0 text-text-secondary" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
								<path stroke-linecap="round" stroke-linejoin="round" d="M9.568 3H5.25A2.25 2.25 0 003 5.25v4.318c0 .597.237 1.17.659 1.591l9.581 9.581c.699.699 1.78.872 2.607.33a18.095 18.095 0 005.223-5.223c.542-.827.369-1.908-.33-2.607L11.16 3.66A2.25 2.25 0 009.568 3z" />
							</svg>
							<span class="font-mono text-sm text-text-primary">{tag.name}</span>
						</div>
						<a
							href="{basePath}/commit/{tag.target}"
							class="shrink-0 font-mono text-xs text-text-secondary transition-colors hover:text-accent"
							title={tag.target}
						>
							{truncateHash(tag.target)}
						</a>
					</li>
				{/each}
			</ul>
		</div>
	{/if}
</section>
