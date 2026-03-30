<script lang="ts">
	import IssueCard from '$lib/components/issue/IssueCard.svelte';
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
		{ label: 'Issues' }
	]);
</script>

<svelte:head>
	<title>Issues - {data.repo} - Cospan</title>
</svelte:head>

<KeyboardShortcuts {basePath} />

<section>
	<Breadcrumb {crumbs} />

	<div class="mt-3 mb-6 flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
		<h1 class="text-xl font-semibold text-text-primary">Issues</h1>
		<a
			href="{basePath}/issues/new"
			class="rounded-md bg-accent px-3.5 py-2 text-center text-sm font-medium text-white transition-colors hover:bg-accent-hover"
		>
			New issue
		</a>
	</div>

	<RepoTabBar {basePath} activeTab="issues" />

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
		<EmptyState
			icon="inbox"
			message="No {data.filterState} issues."
			ctaHref="{basePath}/issues/new"
			ctaLabel="Create your first issue"
		/>
	{:else}
		<div class="space-y-3">
			{#each data.issues.items as issue (issue.rkey)}
				<IssueCard {issue} {basePath} />
			{/each}
		</div>
	{/if}

	<BackLink href={basePath} />
</section>
