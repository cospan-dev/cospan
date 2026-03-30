<script lang="ts">
	import { onMount } from 'svelte';
	import type { Repo } from '$lib/api/repo.js';
	import { resolveHandle } from '$lib/api/handle.js';
	import { getAuth } from '$lib/stores/auth.svelte';

	let { data } = $props();

	let auth = $derived(getAuth());
	let searchQuery = $state('');
	let showMineOnly = $state(false);

	// Resolve handles for all repos
	let handles = $state<Record<string, string>>({});

	onMount(async () => {
		const repos = data.repos.items;
		const unique = [...new Set(repos.map((r: Repo) => r.did))];
		const resolved: Record<string, string> = {};
		await Promise.allSettled(
			unique.map(async (did) => {
				resolved[did] = await resolveHandle(did);
			})
		);
		handles = resolved;
	});

	function getHandle(did: string): string {
		return handles[did] || (did.startsWith('did:plc:') ? did.slice(8, 18) + '…' : did);
	}

	let filteredRepos = $derived(() => {
		let repos = data.repos.items;

		// Filter to user's repos
		if (showMineOnly && auth.authenticated && auth.did) {
			repos = repos.filter((r: Repo) => r.did === auth.did);
		}

		// Search filter
		if (searchQuery.trim()) {
			const q = searchQuery.toLowerCase();
			repos = repos.filter((r: Repo) =>
				r.name.toLowerCase().includes(q) ||
				(r.description ?? '').toLowerCase().includes(q) ||
				getHandle(r.did).toLowerCase().includes(q)
			);
		}

		return repos;
	});

	function tangledUrl(repo: Repo): string {
		const host = repo.nodeUrl || '';
		if (host) return `${host}/${repo.did}/${repo.name}`;
		return `https://tangled.sh/${repo.did}/${repo.name}`;
	}
</script>

<svelte:head>
	<title>Import from Tangled — Cospan</title>
</svelte:head>

<section>
	<div class="mb-6">
		<h1 class="mb-1 text-lg font-semibold text-ink">Import from Tangled</h1>
		<p class="text-[13px] text-caption">
			Browse repositories from the Tangled network and import them into Cospan for schema-aware version control.
		</p>
	</div>

	<!-- Search + filters -->
	<div class="mb-6 flex flex-wrap items-center gap-3">
		<div class="relative flex-1">
			<svg class="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-ghost" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
				<path stroke-linecap="round" stroke-linejoin="round" d="M21 21l-5.197-5.197m0 0A7.5 7.5 0 105.196 5.196a7.5 7.5 0 0010.607 10.607z" />
			</svg>
			<input
				type="text"
				bind:value={searchQuery}
				placeholder="Search Tangled repos…"
				class="w-full rounded-md border border-line bg-surface py-2 pl-10 pr-3 text-[13px] text-ink placeholder:text-ghost focus:border-focus/50 focus:outline-none transition-colors"
			/>
		</div>

		{#if auth.authenticated}
			<button
				type="button"
				onclick={() => showMineOnly = !showMineOnly}
				class="rounded-md px-3 py-2 text-[12px] font-medium transition-all
					{showMineOnly ? 'bg-focus/10 text-focus-bright border border-focus/30' : 'bg-surface text-ghost border border-line hover:text-caption'}"
			>
				My repos
			</button>
		{/if}

		<span class="text-[11px] text-ghost">
			{filteredRepos().length} repos
		</span>
	</div>

	<!-- Repo list -->
	{#if filteredRepos().length === 0}
		<div class="flex flex-col items-center justify-center py-24 text-center">
			<div class="mb-4 text-ghost">
				<svg class="mx-auto h-10 w-10" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1">
					<path stroke-linecap="round" stroke-linejoin="round" d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4" />
				</svg>
			</div>
			{#if searchQuery.trim()}
				<p class="text-sm text-caption">No repos matching "{searchQuery}"</p>
			{:else if showMineOnly}
				<p class="text-sm text-caption">No Tangled repos found for your account.</p>
			{:else}
				<p class="text-sm text-caption">No Tangled repositories found yet.</p>
				<p class="mt-1 text-xs text-ghost">Repositories are being backfilled from the network. Check back soon.</p>
			{/if}
		</div>
	{:else}
		<div class="divide-y divide-line/40">
			{#each filteredRepos() as repo (repo.did + '/' + repo.name)}
				<div class="flex items-start justify-between gap-4 py-4">
					<div class="min-w-0 flex-1">
						<div class="flex items-center gap-2">
							<a href="/{repo.did}/{repo.name}" class="font-mono text-[14px] font-medium text-ink hover:text-focus-bright transition-colors">
								{getHandle(repo.did)}/{repo.name}
							</a>
							{#if repo.protocol}
								<span class="text-[11px] text-ghost">{repo.protocol}</span>
							{:else}
								<span class="text-[11px] text-ghost">git</span>
							{/if}
						</div>
						{#if repo.description}
							<p class="mt-1 line-clamp-1 text-[13px] text-caption">{repo.description}</p>
						{/if}
						<div class="mt-1.5 flex items-center gap-3 text-[11px] text-ghost">
							{#if repo.starCount > 0}
								<span>★ {repo.starCount}</span>
							{/if}
							{#if repo.openIssueCount > 0}
								<span>{repo.openIssueCount} issues</span>
							{/if}
							<a href={tangledUrl(repo)} target="_blank" rel="noopener" class="hover:text-caption transition-colors">
								View on Tangled ↗
							</a>
						</div>
					</div>

					<button
						class="shrink-0 rounded-md border border-focus/40 bg-focus/5 px-3 py-1.5 text-[12px] font-medium text-focus-bright transition-all hover:bg-focus/10 hover:border-focus"
					>
						Import
					</button>
				</div>
			{/each}
		</div>
	{/if}
</section>
