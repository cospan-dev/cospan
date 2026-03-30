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
		const handle = getHandle(repo.did);
		return `https://tangled.sh/${handle}/${repo.name}`;
	}
</script>

<svelte:head>
	<title>Import from Tangled · Cospan</title>
</svelte:head>

<section>
	<div class="mb-5 flex items-end justify-between">
		<div>
			<h1 class="mb-1 text-lg font-semibold text-ink">Import from Tangled</h1>
			<p class="text-[13px] text-caption">Bring repositories into Cospan for schema-aware version control.</p>
		</div>
		{#if !auth.authenticated}
			<span class="text-[12px] text-ghost">Sign in to import your repos</span>
		{/if}
	</div>

	<!-- Your repos (if logged in and have any) -->
	{#if auth.authenticated && myRepos().length > 0}
		<div class="mb-6">
			<div class="mb-2 text-[11px] font-medium uppercase tracking-wider text-ghost">Your repos</div>
			{#each myRepos() as repo (repo.did + '/' + repo.name)}
				<div class="flex items-center justify-between gap-3 rounded-md border border-focus/20 bg-focus/[0.03] px-3 py-2 mb-1.5">
					<div class="flex items-center gap-2 min-w-0">
						<span class="font-mono text-[13px] font-medium text-ink">{repo.name}</span>
						{#if repo.description}
							<span class="truncate text-[12px] text-ghost">· {repo.description}</span>
						{/if}
					</div>
					<button class="shrink-0 rounded-md bg-focus px-3 py-1 text-[12px] font-medium text-white transition-all hover:bg-focus-bright">
						Import
					</button>
				</div>
			{/each}
		</div>
	{/if}

	<!-- Search + count -->
	<div class="mb-3 flex items-center gap-3">
		<div class="relative flex-1">
			<svg class="pointer-events-none absolute left-3 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-ghost" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
				<path stroke-linecap="round" stroke-linejoin="round" d="M21 21l-5.197-5.197m0 0A7.5 7.5 0 105.196 5.196a7.5 7.5 0 0010.607 10.607z" />
			</svg>
			<input
				type="text"
				bind:value={searchQuery}
				placeholder="Search repos…"
				class="w-full rounded-md border border-line bg-surface py-1.5 pl-9 pr-3 text-[13px] text-ink placeholder:text-ghost focus:border-focus/50 focus:outline-none transition-colors"
			/>
		</div>
		<span class="text-[11px] text-ghost whitespace-nowrap">{otherRepos().length} repos</span>
	</div>

	<!-- Repo list -->
	{#if otherRepos().length === 0}
		<div class="py-16 text-center">
			{#if searchQuery.trim()}
				<p class="text-[13px] text-caption">No repos matching "{searchQuery}"</p>
			{:else}
				<p class="text-[13px] text-caption">No Tangled repositories found yet.</p>
				<p class="mt-1 text-[12px] text-ghost">Repositories are being backfilled from the network.</p>
			{/if}
		</div>
	{:else}
		<div class="divide-y divide-line/30">
			{#each otherRepos() as repo (repo.did + '/' + repo.name)}
				<div class="flex items-center justify-between gap-4 py-2.5">
					<div class="min-w-0 flex-1">
						<div class="flex items-center gap-2">
							<a href="/{repo.did}/{repo.name}" class="font-mono text-[13px] font-medium text-ink hover:text-focus-bright transition-colors">
								{getHandle(repo.did)}<span class="text-ghost">/</span>{repo.name}
							</a>
							<span class="text-[11px] text-ghost">{repo.protocol || 'git'}</span>
							{#if repo.starCount > 0}
								<span class="text-[11px] text-ghost">★ {repo.starCount}</span>
							{/if}
						</div>
						{#if repo.description}
							<p class="mt-0.5 truncate text-[12px] text-ghost">{repo.description}</p>
						{/if}
						<a href={tangledUrl(repo)} target="_blank" rel="noopener" class="mt-0.5 block text-[11px] text-ghost hover:text-caption transition-colors">
							View on Tangled ↗
						</a>
					</div>
					<button class="shrink-0 rounded-md border border-line px-2.5 py-1 text-[11px] font-medium text-caption transition-all hover:border-line-bright hover:text-ink">
						Fork
					</button>
				</div>
			{/each}
		</div>
	{/if}
</section>
