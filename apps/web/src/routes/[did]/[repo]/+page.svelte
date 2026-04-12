<script lang="ts">
	import { onMount } from 'svelte';
	import CommitList from '$lib/components/repo/CommitList.svelte';
	import CommitGraph from '$lib/components/repo/CommitGraph.svelte';
	import SchemaHealthCard from '$lib/components/repo/SchemaHealthCard.svelte';
	import SchemaSparkline from '$lib/components/repo/SchemaSparkline.svelte';
	import StarButton from '$lib/components/shared/StarButton.svelte';
	import ForkButton from '$lib/components/shared/ForkButton.svelte';
	import { getAuth } from '$lib/stores/auth.svelte';
	import { resolveHandle } from '$lib/api/handle.js';
	import type { RefUpdateView } from '$lib/generated/views.js';

	let { data } = $props();

	let auth = $derived(getAuth());
	let handle = $state('');

	onMount(async () => {
		const did = data.repo?.did ?? data.did;
		handle = await resolveHandle(did);
	});

	let ownerLabel = $derived(handle || data.repo?.did || data.did);

	let basePath = $derived(data.repo ? `/${data.repo.did}/${data.repo.name}` : `/${data.did}/${data.repoName}`);

	let isTangled = $derived(data.repo?.source === 'tangled');

	let refItems = $derived((data.refUpdates?.items ?? []) as RefUpdateView[]);

	let breakingChangeCount = $derived(
		refItems.reduce((sum, r) => sum + (r.breakingChangeCount ?? 0), 0)
	);

	let lensQuality = $derived.by(() => {
		const withLens = refItems.find((r) => r.lensQuality != null);
		return withLens?.lensQuality ?? null;
	});
</script>

<svelte:head>
	<title>{data.repo ? data.repo.name : data.repoName} · Cospan</title>
</svelte:head>

{#if data.repo}
	<div class="mb-6">
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
						Fork to Cospan
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

	<!-- Schema intelligence: project overview + evolution sparkline -->
	<SchemaHealthCard
		projectSchema={data.projectSchema}
		schemaStats={data.schemaStats}
	/>

	{#if (data.schemaStats?.commits?.length ?? 0) > 1}
		<div class="mb-4 rounded-lg border border-border bg-surface-1 px-4 py-3">
			<h3 class="mb-2 text-xs font-semibold uppercase tracking-wider text-text-muted">
				Schema Evolution
			</h3>
			<SchemaSparkline stats={data.schemaStats.commits} />
		</div>
	{/if}

	<!-- Legacy lens quality / breaking changes from firehose ref updates -->
	{#if lensQuality !== null}
		<div class="mb-4 flex items-center gap-4 text-sm text-text-secondary">
			<span>
				Lens quality: <span class="font-mono font-medium text-text-primary">{(lensQuality * 100).toFixed(0)}%</span>
			</span>
		</div>
	{/if}

	{#if isTangled}
		{@const tangledUrl = `https://tangled.sh/${ownerLabel}/${data.repo.name}`}
		<!-- Tangled repos: browse files on Tangled -->
		<a
			href={tangledUrl}
			class="mb-6 flex items-center gap-3 rounded-lg border border-border bg-surface-1 px-4 py-3 transition-colors hover:border-accent group"
			target="_blank"
			rel="noopener noreferrer"
		>
			<svg class="h-5 w-5 shrink-0 text-text-muted group-hover:text-accent" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
				<path stroke-linecap="round" stroke-linejoin="round" d="M2.25 12.75V12A2.25 2.25 0 014.5 9.75h15A2.25 2.25 0 0121.75 12v.75m-8.69-6.44l-2.12-2.12a1.5 1.5 0 00-1.061-.44H4.5A2.25 2.25 0 002.25 6v12a2.25 2.25 0 002.25 2.25h15A2.25 2.25 0 0021.75 18V9a2.25 2.25 0 00-2.25-2.25h-5.379a1.5 1.5 0 01-1.06-.44z" />
			</svg>
			<div class="min-w-0 flex-1">
				<p class="text-sm font-medium text-text-primary group-hover:text-accent">Browse files on Tangled</p>
				<p class="text-xs text-text-muted truncate">{tangledUrl}</p>
			</div>
			<svg class="h-4 w-4 shrink-0 text-text-muted group-hover:text-accent" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
				<path stroke-linecap="round" stroke-linejoin="round" d="M13.5 6H5.25A2.25 2.25 0 003 8.25v10.5A2.25 2.25 0 005.25 21h10.5A2.25 2.25 0 0018 18.75V10.5m-10.5 6L21 3m0 0h-5.25M21 3v5.25" />
			</svg>
		</a>
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

	{#if (data.commits?.length ?? 0) > 0}
		<!-- Commit graph: full DAG with lanes + merge markers, fetched from
		     the hosting node. Only available for Cospan-hosted repos. -->
		<div class="rounded-lg border border-border bg-surface-1 p-4">
			<h2 class="mb-3 text-sm font-medium text-text-primary">
				Commit graph
				<span class="ml-1 text-xs font-normal text-text-muted">({data.commits.length})</span>
			</h2>
			<CommitGraph
				commits={data.commits}
				{basePath}
				commitUrlBase={`${basePath}/commit`}
			/>
		</div>
	{:else}
		<!-- Fallback: flat list of ref updates from the firehose. Used for
		     Tangled-hosted repos where we only observe pushes via the
		     jetstream, not the full commit DAG. -->
		<div class="rounded-lg border border-border bg-surface-1 p-4">
			<h2 class="mb-3 text-sm font-medium text-text-primary">Recent Activity</h2>
			<CommitList
				refUpdates={data.refUpdates.items}
				commitUrlBase={isTangled
					? `https://tangled.sh/${ownerLabel}/${data.repo.name}/commit`
					: `${basePath}/commit`}
			/>
		</div>
	{/if}
{:else}
	<div class="mb-6">
		<h1 class="mt-3 text-xl font-semibold text-text-primary">{data.repoName}</h1>
		<p class="mt-2 text-sm text-text-secondary">Repository could not be loaded.</p>
	</div>
{/if}
