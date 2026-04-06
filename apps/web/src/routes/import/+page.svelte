<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import type { Repo } from '$lib/api/repo.js';
	import { listRepos } from '$lib/api/repo.js';
	import { resolveHandle } from '$lib/api/handle.js';
	import { xrpcProcedure } from '$lib/api/client.js';
	import { getAuth } from '$lib/stores/auth.svelte';

	let { data } = $props();

	let auth = $derived(getAuth());
	let forkingRepo = $state<string | null>(null);
	let forkError = $state<string | null>(null);

	async function forkRepo(repo: Repo) {
		if (!auth.authenticated || !auth.did) return;
		forkingRepo = `${repo.did}/${repo.name}`;
		forkError = null;
		try {
			const sourceUri = `at://${repo.did}/dev.cospan.repo/${repo.rkey ?? repo.name}`;
			await xrpcProcedure('dev.cospan.repo.fork', {
				sourceRepo: sourceUri,
				did: auth.did,
			});
			goto(`/${auth.did}/${repo.name}`);
		} catch (e: any) {
			forkError = e.message ?? 'Fork failed';
			forkingRepo = null;
		}
	}
	let searchQuery = $state('');
	let activeTab = $state<'all' | 'mine'>('all');

	let handles = $state<Record<string, string>>({});
	let loadingMore = $state(false);
	let allRepos = $state<Repo[]>(data.repos.items);
	let cursor = $state<string | null>(data.repos.cursor);

	// User's repos (fetched separately when logged in)
	let myRepos = $state<Repo[]>([]);
	let myReposLoaded = $state(false);
	let mySearchQuery = $state('');

	onMount(async () => {
		resolveAllHandles(allRepos);
		if (auth.authenticated && auth.did) {
			await loadMyRepos();
		}
	});

	// Re-check when auth becomes available
	$effect(() => {
		if (auth.authenticated && auth.did && !myReposLoaded) {
			loadMyRepos();
		}
	});

	async function loadMyRepos() {
		if (!auth.did) return;
		try {
			const result = await listRepos({ did: auth.did, source: 'tangled', limit: 100 });
			myRepos = result.items;
			myReposLoaded = true;
			resolveAllHandles(myRepos);
		} catch {
			myReposLoaded = true;
		}
	}

	async function resolveAllHandles(repos: Repo[]) {
		const unique = [...new Set(repos.map((r) => r.did))].filter((d) => !handles[d]);
		if (unique.length === 0) return;
		const resolved: Record<string, string> = { ...handles };
		await Promise.allSettled(
			unique.map(async (did) => {
				resolved[did] = await resolveHandle(did);
			})
		);
		handles = resolved;
	}

	let searching = $state(false);
	let searchTimer: ReturnType<typeof setTimeout> | null = null;

	// React to search query changes with debounce
	let mounted = false;
	onMount(() => { mounted = true; });
	$effect(() => {
		const q = searchQuery;
		if (!mounted) return;
		if (searchTimer) clearTimeout(searchTimer);
		searchTimer = setTimeout(() => {
			doSearch(q);
		}, 300);
	});

	async function loadMore() {
		if (!cursor || loadingMore) return;
		loadingMore = true;
		try {
			const result = await listRepos({ source: 'tangled', sort: 'popular', limit: 30, cursor });
			allRepos = [...allRepos, ...result.items];
			cursor = result.cursor;
			resolveAllHandles(result.items);
		} finally {
			loadingMore = false;
		}
	}

	async function doSearch(query: string) {
		if (!query.trim()) {
			// Reset to default popular list
			searching = true;
			try {
				const result = await listRepos({ source: 'tangled', sort: 'popular', limit: 30 });
				allRepos = result.items;
				cursor = result.cursor;
				resolveAllHandles(result.items);
			} finally {
				searching = false;
			}
			return;
		}
		searching = true;
		try {
			const result = await listRepos({ query, source: 'tangled', limit: 30 });
			allRepos = result.items;
			cursor = result.cursor;
			resolveAllHandles(result.items);
		} finally {
			searching = false;
		}
	}

	function getHandle(did: string): string {
		return handles[did] || (did.startsWith('did:plc:') ? did.slice(8, 18) + '\u2026' : did);
	}

	let filteredMine = $derived(() => {
		if (!mySearchQuery.trim()) return myRepos;
		const q = mySearchQuery.toLowerCase();
		return myRepos.filter((r: Repo) =>
			r.name.toLowerCase().includes(q) ||
			(r.description ?? '').toLowerCase().includes(q)
		);
	});

	function tangledUrl(repo: Repo): string {
		const handle = getHandle(repo.did);
		return `https://tangled.sh/${handle}/${repo.name}`;
	}
</script>

<svelte:head>
	<title>Fork from Tangled · Cospan</title>
</svelte:head>

<section>
	<div class="mb-5 flex items-end justify-between">
		<div>
			<h1 class="mb-1 text-lg font-semibold text-ink">Fork from Tangled</h1>
			<p class="text-[13px] text-caption">Fork Tangled repositories into Cospan for schematic version control.</p>
		</div>
		{#if !auth.authenticated}
			<span class="text-[12px] text-ghost">Sign in to see your repos</span>
		{/if}
	</div>

	<!-- Tabs (when logged in) -->
	{#if auth.authenticated}
		<div class="mb-4 flex items-center gap-0.5 rounded-lg bg-surface p-1 w-fit">
			<button
				type="button"
				onclick={() => activeTab = 'mine'}
				class="rounded-md px-3 py-1.5 text-[12px] font-medium transition-all
					{activeTab === 'mine' ? 'bg-raised text-ink shadow-sm' : 'text-ghost hover:text-caption'}"
			>
				Your Repos
			</button>
			<button
				type="button"
				onclick={() => activeTab = 'all'}
				class="rounded-md px-3 py-1.5 text-[12px] font-medium transition-all
					{activeTab === 'all' ? 'bg-raised text-ink shadow-sm' : 'text-ghost hover:text-caption'}"
			>
				All Repos
			</button>
		</div>
	{/if}

	<!-- ═══ YOUR REPOS TAB ═══ -->
	{#if auth.authenticated && activeTab === 'mine'}
		<div class="mb-3 flex items-center gap-3">
			<div class="relative flex-1">
				<svg class="pointer-events-none absolute left-3 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-ghost" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
					<path stroke-linecap="round" stroke-linejoin="round" d="M21 21l-5.197-5.197m0 0A7.5 7.5 0 105.196 5.196a7.5 7.5 0 0010.607 10.607z" />
				</svg>
				<input
					type="text"
					bind:value={mySearchQuery}
					placeholder="Search your repos…"
					class="w-full rounded-md border border-line bg-surface py-1.5 pl-9 pr-3 text-[13px] text-ink placeholder:text-ghost focus:border-focus/50 focus:outline-none transition-colors"
				/>
			</div>
			<span class="text-[11px] text-ghost whitespace-nowrap">{filteredMine().length} repos</span>
		</div>

		{#if !myReposLoaded}
			<div class="py-16 text-center">
				<p class="text-[13px] text-caption">Loading your repos…</p>
			</div>
		{:else if filteredMine().length === 0}
			<div class="py-16 text-center">
				{#if mySearchQuery.trim()}
					<p class="text-[13px] text-caption">No repos matching "{mySearchQuery}"</p>
				{:else}
					<p class="text-[13px] text-caption">You don't have any Tangled repositories yet.</p>
				{/if}
			</div>
		{:else}
			<div class="divide-y divide-line/30">
				{#each filteredMine() as repo (repo.did + '/' + repo.name)}
					<div class="flex items-center justify-between gap-3 py-2.5">
						<div class="min-w-0 flex-1">
							<div class="flex items-center gap-2">
								<a href="/{repo.did}/{repo.name}" class="font-mono text-[13px] font-medium text-ink hover:text-focus-bright transition-colors">
									{repo.name}
								</a>
								{#if repo.starCount > 0}
									<span class="text-[11px] text-ghost">★ {repo.starCount}</span>
								{/if}
							</div>
							{#if repo.description}
								<p class="mt-0.5 truncate text-[12px] text-ghost">{repo.description}</p>
							{/if}
						</div>
						<button
							onclick={() => forkRepo(repo)}
							disabled={forkingRepo === `${repo.did}/${repo.name}`}
							class="shrink-0 rounded-md bg-focus px-3 py-1 text-[12px] font-medium text-white transition-all hover:bg-focus-bright disabled:opacity-50"
						>
							{forkingRepo === `${repo.did}/${repo.name}` ? 'Forking…' : 'Fork'}
						</button>
					</div>
				{/each}
			</div>
		{/if}
	{:else}
		<!-- ═══ ALL REPOS TAB ═══ -->
		<div class="mb-3 flex items-center gap-3">
			<div class="relative flex-1">
				<svg class="pointer-events-none absolute left-3 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-ghost" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
					<path stroke-linecap="round" stroke-linejoin="round" d="M21 21l-5.197-5.197m0 0A7.5 7.5 0 105.196 5.196a7.5 7.5 0 0010.607 10.607z" />
				</svg>
				<input
					type="text"
					bind:value={searchQuery}
					placeholder="Search all repos…"
					class="w-full rounded-md border border-line bg-surface py-1.5 pl-9 pr-3 text-[13px] text-ink placeholder:text-ghost focus:border-focus/50 focus:outline-none transition-colors"
				/>
			</div>
			<span class="text-[11px] text-ghost whitespace-nowrap">{searching ? 'Searching…' : `${allRepos.length} repos`}</span>
		</div>

		{#if allRepos.length === 0}
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
				{#each allRepos as repo (repo.did + '/' + repo.name)}
					<div class="flex items-center justify-between gap-4 py-2.5">
						<div class="min-w-0 flex-1">
							<div class="flex items-center gap-2">
								<a href="/{repo.did}/{repo.name}" class="font-mono text-[13px] font-medium text-ink hover:text-focus-bright transition-colors">
									{getHandle(repo.did)}<span class="text-ghost">/</span>{repo.name}
								</a>
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
						<button
							onclick={() => forkRepo(repo)}
							disabled={!auth.authenticated || forkingRepo === `${repo.did}/${repo.name}`}
							class="shrink-0 rounded-md border border-line px-2.5 py-1 text-[11px] font-medium text-caption transition-all hover:border-line-bright hover:text-ink disabled:opacity-50"
						>
							{forkingRepo === `${repo.did}/${repo.name}` ? 'Forking…' : 'Fork'}
						</button>
					</div>
				{/each}
			</div>

			<!-- Load more -->
			{#if cursor && !searchQuery.trim()}
				<div class="mt-4 text-center">
					<button
						type="button"
						onclick={loadMore}
						disabled={loadingMore}
						class="rounded-md border border-line px-4 py-2 text-[12px] font-medium text-caption transition-all hover:border-line-bright hover:text-ink disabled:opacity-50"
					>
						{loadingMore ? 'Loading…' : 'Load more'}
					</button>
				</div>
			{/if}
		{/if}
	{/if}
</section>
