<script lang="ts">
	import CommitList from '$lib/components/repo/CommitList.svelte';
	import Breadcrumb from '$lib/components/shared/Breadcrumb.svelte';
	import KeyboardShortcuts from '$lib/components/shared/KeyboardShortcuts.svelte';
	import StarButton from '$lib/components/shared/StarButton.svelte';
	import ForkButton from '$lib/components/shared/ForkButton.svelte';
	import { getAuth } from '$lib/stores/auth.svelte';

	let { data } = $props();

	let auth = $derived(getAuth());
	let isOwner = $derived(auth.authenticated && data.repo?.did === auth.did);

	let basePath = $derived(data.repo ? `/${data.repo.did}/${data.repo.name}` : `/${data.did}/${data.repoName}`);

	let crumbs = $derived(data.repo
		? [
				{ label: data.repo.did, href: `/${data.repo.did}` },
				{ label: data.repo.name }
			]
		: [
				{ label: data.did, href: `/${data.did}` },
				{ label: data.repoName }
			]
	);
</script>

<svelte:head>
	<title>{data.repo ? data.repo.name : data.repoName} - Cospan</title>
</svelte:head>

<KeyboardShortcuts {basePath} />

<section>
	{#if data.repo}
		<div class="mb-6">
			<Breadcrumb {crumbs} />

			<div class="mt-3 flex items-center justify-between">
				<div class="flex items-center gap-3">
					<h1 class="text-xl font-semibold text-text-primary">{data.repo.name}</h1>
					<span class="rounded-full bg-surface-2 px-2 py-0.5 font-mono text-xs text-text-secondary">
						{data.repo.protocol}
					</span>
				</div>
				<div class="flex items-center gap-2">
					<ForkButton repoDid={data.repo.did} repoName={data.repo.name} />
					<StarButton subject={`at://${data.repo.did}/dev.cospan.repo/${data.repo.name}`} starCount={data.repo.starCount} />
				</div>
			</div>

			<div class="mt-2 flex items-center gap-3">
				{#if data.repo.openIssueCount > 0}
					<span class="text-xs text-text-secondary">{data.repo.openIssueCount} issues</span>
				{/if}
				{#if data.repo.openMrCount > 0}
					<span class="text-xs text-text-secondary">{data.repo.openMrCount} MRs</span>
				{/if}
			</div>

			{#if data.repo.description}
				<p class="mt-3 text-sm text-text-secondary">{data.repo.description}</p>
			{/if}
		</div>

		<div class="mb-6 flex items-center gap-1 border-b border-border overflow-x-auto">
			<a
				href={basePath}
				class="border-b-2 border-accent px-4 py-2 text-sm font-medium text-text-primary"
			>
				Code
			</a>
			<a
				href="{basePath}/issues"
				class="border-b-2 border-transparent px-4 py-2 text-sm text-text-secondary hover:text-text-primary transition-colors"
			>
				Issues
				{#if data.repo.openIssueCount > 0}
					<span class="ml-1 rounded-full bg-surface-2 px-1.5 py-0.5 text-xs">
						{data.repo.openIssueCount}
					</span>
				{/if}
			</a>
			<a
				href="{basePath}/pulls"
				class="border-b-2 border-transparent px-4 py-2 text-sm text-text-secondary hover:text-text-primary transition-colors"
			>
				Merge Requests
				{#if data.repo.openMrCount > 0}
					<span class="ml-1 rounded-full bg-surface-2 px-1.5 py-0.5 text-xs">
						{data.repo.openMrCount}
					</span>
				{/if}
			</a>
			<a
				href="{basePath}/branches"
				class="border-b-2 border-transparent px-4 py-2 text-sm text-text-secondary hover:text-text-primary transition-colors"
			>
				Branches
			</a>
			<a
				href="{basePath}/tags"
				class="border-b-2 border-transparent px-4 py-2 text-sm text-text-secondary hover:text-text-primary transition-colors"
			>
				Tags
			</a>
			<a
				href="{basePath}/compare"
				class="border-b-2 border-transparent px-4 py-2 text-sm text-text-secondary hover:text-text-primary transition-colors"
			>
				Compare
			</a>
			{#if isOwner}
				<a
					href="{basePath}/settings"
					class="border-b-2 border-transparent px-4 py-2 text-sm text-text-secondary hover:text-text-primary transition-colors"
				>
					Settings
				</a>
			{/if}
		</div>

		<!-- Browse Code button -->
		<div class="mb-6">
			<a
				href="{basePath}/tree"
				class="inline-flex items-center gap-2 rounded-lg border border-border bg-surface-1 px-4 py-2.5 text-sm text-text-primary transition-colors hover:border-accent"
			>
				<svg class="h-4 w-4 text-text-secondary" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
					<path stroke-linecap="round" stroke-linejoin="round" d="M2.25 12.75V12A2.25 2.25 0 014.5 9.75h15A2.25 2.25 0 0121.75 12v.75m-8.69-6.44l-2.12-2.12a1.5 1.5 0 00-1.061-.44H4.5A2.25 2.25 0 002.25 6v12a2.25 2.25 0 002.25 2.25h15A2.25 2.25 0 0021.75 18V9a2.25 2.25 0 00-2.25-2.25h-5.379a1.5 1.5 0 01-1.06-.44z" />
				</svg>
				Browse Code
			</a>
		</div>

		<div class="rounded-lg border border-border bg-surface-1 p-4">
			<h2 class="mb-3 text-sm font-medium text-text-primary">Recent Activity</h2>
			<CommitList refUpdates={data.refUpdates.items} />
		</div>
	{:else}
		<div class="mb-6">
			<Breadcrumb {crumbs} />
			<h1 class="mt-3 text-xl font-semibold text-text-primary">{data.repoName}</h1>
			<p class="mt-2 text-sm text-text-secondary">Repository could not be loaded.</p>
		</div>
	{/if}
</section>
