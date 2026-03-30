<script lang="ts">
	import { goto } from '$app/navigation';
	import RepoCard from '$lib/components/repo/RepoCard.svelte';
	import { debounce } from '$lib/utils/debounce.js';

	let { data } = $props();

	let initialQuery = data.query ?? '';
	let inputValue = $state(initialQuery);

	const debouncedSearch = debounce((q: string) => {
		if (q.trim()) {
			goto(`/search?q=${encodeURIComponent(q.trim())}`, { keepFocus: true });
		}
	}, 300);

	function handleInput(event: Event) {
		const target = event.target as HTMLInputElement;
		inputValue = target.value;
		debouncedSearch(inputValue);
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter' && inputValue.trim()) {
			goto(`/search?q=${encodeURIComponent(inputValue.trim())}`, { keepFocus: true });
		}
	}
</script>

<svelte:head>
	<title>{data.query ? `Search: ${data.query}` : 'Search'} · Cospan</title>
</svelte:head>

<section>
	<div class="mb-6">
		<h1 class="mb-1 text-lg font-semibold text-ink">Search</h1>
		<p class="mb-4 text-[13px] text-caption">Find repositories across Cospan.</p>
		<input
			type="text"
			value={inputValue}
			oninput={handleInput}
			onkeydown={handleKeydown}
			placeholder="Search repositories…"
			class="w-full rounded-md border border-line bg-surface px-3 py-2 text-[14px] text-ink placeholder:text-ghost focus:border-focus/50 focus:outline-none transition-colors"
		/>
	</div>

	{#if data.results === null}
		<div class="flex flex-col items-center gap-4 py-16 text-text-muted">
			<svg class="h-12 w-12" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
				<path stroke-linecap="round" stroke-linejoin="round" d="M21 21l-5.197-5.197m0 0A7.5 7.5 0 105.196 5.196a7.5 7.5 0 0010.607 10.607z" />
			</svg>
			<p class="text-sm">Enter a search query to find repositories.</p>
		</div>
	{:else if data.results.items.length === 0}
		<div class="flex flex-col items-center gap-4 py-16 text-text-muted">
			<svg class="h-12 w-12" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
				<path stroke-linecap="round" stroke-linejoin="round" d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4" />
			</svg>
			<p class="text-sm">No repositories found for "{data.query}".</p>
		</div>
	{:else}
		<p class="mb-4 text-sm text-text-muted">
			{data.results.totalCount} {data.results.totalCount === 1 ? 'result' : 'results'} for "{data.query}"
		</p>
		<div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
			{#each data.results.items as repo (repo.did + '/' + repo.name)}
				<RepoCard {repo} />
			{/each}
		</div>
	{/if}
</section>
