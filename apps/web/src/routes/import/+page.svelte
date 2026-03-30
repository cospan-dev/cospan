<script lang="ts">
	import { onMount } from 'svelte';
	import type { Repo } from '$lib/api/repo.js';
	import { resolveHandle } from '$lib/api/handle.js';

	let { data } = $props();

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

	function tangledUrl(repo: Repo): string {
		// Build Tangled web URL from nodeUrl or knot hostname
		const host = repo.nodeUrl || '';
		if (host) {
			return `${host}/${repo.did}/${repo.name}`;
		}
		return `https://tangled.sh/${repo.did}/${repo.name}`;
	}
</script>

<svelte:head>
	<title>Import from Tangled — Cospan</title>
</svelte:head>

<div class="py-2">
	<!-- Header -->
	<div class="mb-8">
		<h1 class="text-2xl font-semibold text-ink">Import from Tangled</h1>
		<p class="mt-2 text-[14px] text-caption">
			Browse repositories from the Tangled network and import them into Cospan for schema-aware version control.
		</p>
	</div>

	<!-- Repo list -->
	{#if data.repos.items.length === 0}
		<div class="flex flex-col items-center justify-center py-24 text-center">
			<div class="mb-4 text-ghost">
				<svg class="mx-auto h-10 w-10" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1">
					<path stroke-linecap="round" stroke-linejoin="round" d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4" />
				</svg>
			</div>
			<p class="text-sm text-caption">No Tangled repositories found yet.</p>
			<p class="mt-1 text-xs text-ghost">Repositories are being backfilled from the network. Check back soon.</p>
		</div>
	{:else}
		<div class="divide-y divide-line/40">
			{#each data.repos.items as repo (repo.did + '/' + repo.name)}
				<div class="flex items-start justify-between gap-4 py-4">
					<!-- Repo info -->
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

					<!-- Import button -->
					<button
						class="shrink-0 rounded-md border border-focus/40 bg-focus/5 px-3 py-1.5 text-[12px] font-medium text-focus-bright transition-all hover:bg-focus/10 hover:border-focus"
					>
						Import
					</button>
				</div>
			{/each}
		</div>
	{/if}
</div>
