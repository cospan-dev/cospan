<script lang="ts">
	import { onMount } from 'svelte';
	import type { Repo } from '$lib/api/repo.js';
	import { resolveHandle } from '$lib/api/handle.js';
	import { getAuth } from '$lib/stores/auth.svelte';

	let { data } = $props();

	let auth = $derived(getAuth());
	let searchQuery = $state('');

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

	let myRepos = $derived(() => {
		if (!auth.authenticated || !auth.did) return [];
		return data.repos.items.filter((r: Repo) => r.did === auth.did);
	});

	let otherRepos = $derived(() => {
		let repos = data.repos.items;
		if (auth.authenticated && auth.did) {
			repos = repos.filter((r: Repo) => r.did !== auth.did);
		}
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
	<title>Import from Tangled · Cospan</title>
</svelte:head>

<section>
	<div class="mb-6">
		<h1 class="mb-1 text-lg font-semibold text-ink">Import from Tangled</h1>
		<p class="text-[13px] text-caption">
			Bring your Tangled repositories into Cospan for schema-aware version control.
		</p>
	</div>

	<!-- Your repos section -->
	{#if auth.authenticated}
		{#if myRepos().length > 0}
			<div class="mb-8">
				<h2 class="mb-3 text-[13px] font-semibold uppercase tracking-wide text-ink">Your Tangled repos</h2>
				<div class="divide-y divide-line/40 rounded-lg border border-line/60 bg-ground">
					{#each myRepos() as repo (repo.did + '/' + repo.name)}
						<div class="flex items-center justify-between gap-4 px-4 py-3">
							<div class="min-w-0 flex-1">
								<div class="flex items-center gap-2">
									<a href="/{repo.did}/{repo.name}" class="font-mono text-[13px] font-medium text-ink hover:text-focus-bright transition-colors">
										{repo.name}
									</a>
									{#if repo.description}
										<span class="truncate text-[12px] text-ghost">{repo.description}</span>
									{/if}
								</div>
							</div>
							<button class="shrink-0 rounded-md bg-focus/10 border border-focus/30 px-3 py-1.5 text-[12px] font-medium text-focus-bright transition-all hover:bg-focus/15 hover:border-focus">
								Import
							</button>
						</div>
					{/each}
				</div>
			</div>
		{:else}
			<div class="mb-8 rounded-lg border border-line/40 bg-ground px-4 py-6 text-center">
				<p class="text-[13px] text-caption">You don't have any Tangled repositories yet.</p>
				<p class="mt-1 text-[12px] text-ghost">Create a repo on <a href="https://tangled.sh" target="_blank" rel="noopener" class="text-focus-bright hover:underline">tangled.sh</a> to import it here.</p>
			</div>
		{/if}
	{:else}
		<div class="mb-8 rounded-lg border border-line/40 bg-ground px-4 py-6 text-center">
			<p class="text-[13px] text-caption">Sign in to import your Tangled repositories.</p>
		</div>
	{/if}

	<!-- All Tangled repos -->
	<div>
		<div class="mb-3 flex items-center justify-between gap-3">
			<h2 class="text-[13px] font-semibold uppercase tracking-wide text-ink">Browse all</h2>
			<span class="text-[11px] text-ghost">{otherRepos().length} repos</span>
		</div>

		<div class="relative mb-4">
			<svg class="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-ghost" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
				<path stroke-linecap="round" stroke-linejoin="round" d="M21 21l-5.197-5.197m0 0A7.5 7.5 0 105.196 5.196a7.5 7.5 0 0010.607 10.607z" />
			</svg>
			<input
				type="text"
				bind:value={searchQuery}
				placeholder="Search repos…"
				class="w-full rounded-md border border-line bg-surface py-2 pl-10 pr-3 text-[13px] text-ink placeholder:text-ghost focus:border-focus/50 focus:outline-none transition-colors"
			/>
		</div>

		{#if otherRepos().length === 0}
			<div class="flex flex-col items-center justify-center py-16 text-center">
				{#if searchQuery.trim()}
					<p class="text-sm text-caption">No repos matching "{searchQuery}"</p>
				{:else}
					<div class="mb-4 text-ghost">
						<svg class="mx-auto h-10 w-10" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1">
							<path stroke-linecap="round" stroke-linejoin="round" d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4" />
						</svg>
					</div>
					<p class="text-sm text-caption">No Tangled repositories found yet.</p>
					<p class="mt-1 text-xs text-ghost">Repositories are being backfilled from the network.</p>
				{/if}
			</div>
		{:else}
			<div class="divide-y divide-line/40">
				{#each otherRepos() as repo (repo.did + '/' + repo.name)}
					<div class="flex items-start justify-between gap-4 py-3">
						<div class="min-w-0 flex-1">
							<div class="flex items-center gap-2">
								<a href="/{repo.did}/{repo.name}" class="font-mono text-[13px] font-medium text-ink hover:text-focus-bright transition-colors">
									{getHandle(repo.did)}/{repo.name}
								</a>
								{#if repo.protocol}
									<span class="text-[11px] text-ghost">{repo.protocol}</span>
								{:else}
									<span class="text-[11px] text-ghost">git</span>
								{/if}
							</div>
							{#if repo.description}
								<p class="mt-0.5 line-clamp-1 text-[12px] text-caption">{repo.description}</p>
							{/if}
							<div class="mt-1 flex items-center gap-3 text-[11px] text-ghost">
								{#if repo.starCount > 0}
									<span>★ {repo.starCount}</span>
								{/if}
								<a href={tangledUrl(repo)} target="_blank" rel="noopener" class="hover:text-caption transition-colors">
									View on Tangled ↗
								</a>
							</div>
						</div>

						<button class="shrink-0 rounded-md border border-line px-3 py-1.5 text-[12px] font-medium text-caption transition-all hover:border-line-bright hover:text-ink">
							Fork to Cospan
						</button>
					</div>
				{/each}
			</div>
		{/if}
	</div>
</section>
