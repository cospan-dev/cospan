<script lang="ts">
	import { onMount } from 'svelte';
	import CommitList from '$lib/components/repo/CommitList.svelte';
	import RepoTabBar from '$lib/components/repo/RepoTabBar.svelte';
	import Breadcrumb from '$lib/components/shared/Breadcrumb.svelte';
	import KeyboardShortcuts from '$lib/components/shared/KeyboardShortcuts.svelte';
	import StarButton from '$lib/components/shared/StarButton.svelte';
	import ForkButton from '$lib/components/shared/ForkButton.svelte';
	import { getAuth } from '$lib/stores/auth.svelte';
	import { resolveHandle } from '$lib/api/handle.js';
	import type { RefUpdate } from '$lib/api/ref-update.js';

	let { data } = $props();

	let auth = $derived(getAuth());
	let isOwner = $derived(auth.authenticated && data.repo?.did === auth.did);
	let handle = $state('');

	onMount(async () => {
		const did = data.repo?.did ?? data.did;
		handle = await resolveHandle(did);
	});

	let ownerLabel = $derived(handle || data.repo?.did || data.did);

	let basePath = $derived(data.repo ? `/${data.repo.did}/${data.repo.name}` : `/${data.did}/${data.repoName}`);

	let crumbs = $derived(data.repo
		? [
				{ label: ownerLabel, href: `/${data.repo.did}` },
				{ label: data.repo.name }
			]
		: [
				{ label: ownerLabel, href: `/${data.did}` },
				{ label: data.repoName }
			]
	);

	let isTangled = $derived(data.repo?.source === 'tangled');

	let refItems = $derived((data.refUpdates?.items ?? []) as RefUpdate[]);

	// Check if any refUpdates have algebraic check data
	let hasAlgebraicChecks = $derived(
		refItems.some((r) => r.algebraicChecks)
	);

	let latestChecks = $derived.by(() => {
		const withChecks = refItems.find((r) => r.algebraicChecks);
		return withChecks?.algebraicChecks ?? null;
	});

	let breakingChangeCount = $derived(
		refItems.reduce((sum, r) => sum + (r.breakingChangeCount ?? 0), 0)
	);

	let lensQuality = $derived.by(() => {
		const withLens = refItems.find((r) => r.lensQuality != null);
		return withLens?.lensQuality ?? null;
	});
</script>

<svelte:head>
	<title>{data.repo ? data.repo.name : data.repoName} - Cospan</title>
</svelte:head>

<KeyboardShortcuts {basePath} />

<section>
	{#if data.repo}
		<div class="mb-6">
			<Breadcrumb {crumbs} />

			<div class="mt-3 flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
				<div class="flex items-center gap-3">
					<h1 class="text-xl font-semibold text-text-primary">{data.repo.name}</h1>
					<span class="rounded-full bg-accent/15 px-2.5 py-0.5 font-mono text-xs font-medium text-accent">
						{data.repo.protocol}
					</span>
					{#if isTangled}
						<span class="rounded-full bg-info/15 px-2.5 py-0.5 text-xs font-medium text-info">
							tangled
						</span>
					{/if}
					{#if data.repo.source && !isTangled}
						<span class="rounded-full bg-surface-2 px-2 py-0.5 text-xs text-text-muted">
							{data.repo.source}
						</span>
					{/if}
				</div>
				<div class="flex items-center gap-2">
					{#if isTangled}
						<a
							href="{basePath}/import"
							class="rounded-md border border-accent bg-accent/10 px-3 py-1.5 text-xs font-medium text-accent transition-colors hover:bg-accent/20"
						>
							Import to Cospan
						</a>
					{/if}
					<ForkButton repoDid={data.repo.did} repoName={data.repo.name} />
					<StarButton subject={`at://${data.repo.did}/dev.cospan.repo/${data.repo.name}`} starCount={data.repo.starCount} />
				</div>
			</div>

			<!-- Metadata row -->
			<div class="mt-2 flex flex-wrap items-center gap-3 text-xs text-text-muted">
				{#if data.repo.starCount > 0}
					<span>{data.repo.starCount} stars</span>
				{/if}
				{#if data.repo.forkCount > 0}
					<span>{data.repo.forkCount} forks</span>
				{/if}
				{#if data.repo.openIssueCount > 0}
					<span>{data.repo.openIssueCount} issues</span>
				{/if}
				{#if data.repo.openMrCount > 0}
					<span>{data.repo.openMrCount} MRs</span>
				{/if}
			</div>

			{#if data.repo.description}
				<p class="mt-3 text-sm text-text-secondary leading-relaxed">{data.repo.description}</p>
			{/if}
		</div>

		<!-- Schema Health card -->
		{#if hasAlgebraicChecks || breakingChangeCount > 0 || lensQuality !== null}
			<div class="mb-6 rounded-lg border border-border bg-surface-0 p-4">
				<h3 class="mb-2 text-xs font-medium uppercase tracking-wider text-text-muted">Schema Health</h3>
				<div class="flex flex-wrap items-center gap-4 text-sm">
					{#if latestChecks}
						<span class="{latestChecks.gatTypeCheck === 'pass' ? 'text-success' : 'text-danger'}">
							GAT {latestChecks.gatTypeCheck === 'pass' ? '&#10003;' : '&#10007;'}
						</span>
						<span class="{latestChecks.equationVerification === 'pass' ? 'text-success' : 'text-danger'}">
							Equations {latestChecks.equationVerification === 'pass' ? '&#10003;' : '&#10007;'}
						</span>
						<span class="{latestChecks.lensLawCheck === 'pass' ? 'text-success' : 'text-danger'}">
							Lens Laws {latestChecks.lensLawCheck === 'pass' ? '&#10003;' : '&#10007;'}
						</span>
						<span class="{latestChecks.breakingChangeCheck === 'pass' ? 'text-success' : 'text-danger'}">
							Breaking {latestChecks.breakingChangeCheck === 'pass' ? '&#10003;' : '&#10007;'}
						</span>
					{/if}
					{#if lensQuality !== null}
						<span class="text-text-secondary">
							Lens quality: <span class="font-mono font-medium text-text-primary">{(lensQuality * 100).toFixed(0)}%</span>
						</span>
					{/if}
					{#if breakingChangeCount > 0}
						<span class="text-danger font-medium">
							{breakingChangeCount} breaking {breakingChangeCount === 1 ? 'change' : 'changes'}
						</span>
					{/if}
				</div>
			</div>
		{/if}

		<RepoTabBar
			{basePath}
			activeTab="code"
			openIssueCount={data.repo.openIssueCount}
			openMrCount={data.repo.openMrCount}
			{isOwner}
		/>

		{#if isTangled}
			<!-- Tangled repos: hosted externally -->
			<div class="mb-6 rounded-lg border border-border bg-surface-1 p-6 text-center">
				<p class="text-sm text-text-secondary">
					Code is hosted on
					<a
						href="https://tangled.sh/{data.repo.did}/{data.repo.name}"
						class="font-medium text-accent transition-colors hover:text-accent-hover"
						target="_blank"
						rel="noopener noreferrer"
					>
						Tangled
					</a>
				</p>
			</div>
		{:else}
			<!-- Browse Code button -->
			<div class="mb-6">
				<a
					href="{basePath}/tree"
					class="inline-flex items-center gap-2 rounded-lg border border-border bg-surface-1 px-4 py-2.5 text-sm text-text-primary transition-colors hover:border-accent"
				>
					<svg class="h-4 w-4 text-text-muted" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
						<path stroke-linecap="round" stroke-linejoin="round" d="M2.25 12.75V12A2.25 2.25 0 014.5 9.75h15A2.25 2.25 0 0121.75 12v.75m-8.69-6.44l-2.12-2.12a1.5 1.5 0 00-1.061-.44H4.5A2.25 2.25 0 002.25 6v12a2.25 2.25 0 002.25 2.25h15A2.25 2.25 0 0021.75 18V9a2.25 2.25 0 00-2.25-2.25h-5.379a1.5 1.5 0 01-1.06-.44z" />
					</svg>
					Browse Code
				</a>
			</div>
		{/if}

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
