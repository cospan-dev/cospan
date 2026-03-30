<script lang="ts">
	import { getContext } from 'svelte';
	import BackLink from '$lib/components/shared/BackLink.svelte';
	import EmptyState from '$lib/components/shared/EmptyState.svelte';

	let { data } = $props();

	let basePath = $derived(`/${data.did}/${data.repoName}`);

	const repoLayout = getContext<any>('repoLayout');
	$effect(() => {
		repoLayout?.setExtraCrumbs([{ label: 'Tags' }]);
		return () => repoLayout?.setExtraCrumbs([]);
	});

	function truncateHash(hash: string): string {
		return hash.slice(0, 10);
	}
</script>

<svelte:head>
	<title>Tags · {data.repoName} · Cospan</title>
</svelte:head>

<h1 class="mt-3 mb-6 text-xl font-semibold text-text-primary">Tags</h1>

{#if data.tags.length === 0}
	<EmptyState
		icon="tag"
		message="No tags found."
		description="Tags are created when a release is published."
	/>
{:else}
	<div class="rounded-lg border border-border bg-surface-1">
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

<BackLink href={basePath} />
