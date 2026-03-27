<script lang="ts">
	import IssueCard from '$lib/components/issue/IssueCard.svelte';
	import Breadcrumb from '$lib/components/shared/Breadcrumb.svelte';
	import KeyboardShortcuts from '$lib/components/shared/KeyboardShortcuts.svelte';

	let { data } = $props();

	let basePath = $derived(`/${data.did}/${data.repo}`);

	let crumbs = $derived([
		{ label: data.did, href: `/${data.did}` },
		{ label: data.repo, href: basePath },
		{ label: 'Issues' }
	]);
</script>

<svelte:head>
	<title>Issues - {data.repo} - Cospan</title>
</svelte:head>

<KeyboardShortcuts {basePath} />

<section>
	<div class="mb-6 flex items-center justify-between">
		<div>
			<Breadcrumb {crumbs} />
			<h1 class="mt-3 text-xl font-semibold text-text-primary">Issues</h1>
		</div>
		<a
			href="{basePath}/issues/new"
			class="rounded-md bg-accent px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-accent-hover"
		>
			New issue
		</a>
	</div>

	<div class="mb-4 flex items-center gap-2">
		<a
			href="{basePath}/issues?state=open"
			class="rounded-md px-3 py-1.5 text-sm transition-colors {data.filterState === 'open'
				? 'bg-surface-2 text-text-primary font-medium'
				: 'text-text-secondary hover:text-text-primary'}"
		>
			Open
		</a>
		<a
			href="{basePath}/issues?state=closed"
			class="rounded-md px-3 py-1.5 text-sm transition-colors {data.filterState === 'closed'
				? 'bg-surface-2 text-text-primary font-medium'
				: 'text-text-secondary hover:text-text-primary'}"
		>
			Closed
		</a>
	</div>

	{#if data.issues.items.length === 0}
		<div class="flex flex-col items-center gap-4 py-12 text-text-secondary">
			<svg class="h-12 w-12" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
				<path stroke-linecap="round" stroke-linejoin="round" d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4" />
			</svg>
			<p>No {data.filterState} issues.</p>
		</div>
	{:else}
		<div class="space-y-3">
			{#each data.issues.items as issue (issue.rkey)}
				<IssueCard {issue} {basePath} />
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
