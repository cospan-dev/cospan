<script lang="ts">
	import RepoCard from '$lib/components/repo/RepoCard.svelte';
	import { ALL_LANGUAGES } from '$lib/data/languages';
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';

	let { data } = $props();

	let activeProtocol = $derived(data.protocol);
	let searchQuery = $state('');
	let showDropdown = $state(false);

	// Source tab from URL
	let activeSource = $derived($page.url.searchParams.get('source') ?? 'all');

	let filtered = $derived(
		searchQuery.trim()
			? ALL_LANGUAGES.filter((l) =>
					l.label.toLowerCase().includes(searchQuery.toLowerCase()) ||
					l.value.toLowerCase().includes(searchQuery.toLowerCase())
				).slice(0, 20)
			: ALL_LANGUAGES.slice(0, 20)
	);

	function selectProtocol(value: string) {
		showDropdown = false;
		searchQuery = '';
		goto(`/?protocol=${value}`);
	}

	const sourceTabs = [
		{ value: 'all', label: 'All' },
		{ value: 'cospan', label: 'Cospan' },
		{ value: 'tangled', label: 'Tangled' },
	] as const;

	function setSource(source: string) {
		const params = new URLSearchParams($page.url.searchParams);
		if (source === 'all') {
			params.delete('source');
		} else {
			params.set('source', source);
		}
		const qs = params.toString();
		goto(qs ? `/?${qs}` : '/');
	}

	// Generate a deterministic color from language name
	function langColor(value: string): string {
		let hash = 0;
		for (let i = 0; i < value.length; i++) {
			hash = value.charCodeAt(i) + ((hash << 5) - hash);
		}
		const hue = Math.abs(hash % 360);
		return `oklch(0.65 0.15 ${hue})`;
	}
</script>

<svelte:head>
	<title>Cospan</title>
</svelte:head>

<!-- Hero section -->
<section class="noise-overlay graph-paper relative -mx-4 -mt-6 mb-8 px-4 py-16" style="background: linear-gradient(180deg, oklch(0.14 0.02 270) 0%, oklch(0.11 0.008 270) 100%);">
	<div class="relative mx-auto max-w-6xl">
		<div class="flex items-start justify-between">
			<div>
				<h1 class="text-[28px] font-medium leading-tight text-text-primary">
					Schema-first code hosting.
				</h1>
				<p class="mt-3 text-[15px] leading-relaxed text-text-secondary">
					Structural diffs. Algebraic CI. Breaking change detection.
				</p>
				<p class="mt-4 text-xs text-text-muted">Built on AT Protocol</p>
			</div>

			<!-- Abstract cospan diagram decoration -->
			<svg class="hidden h-24 w-40 md:block" viewBox="0 0 160 96" fill="none" xmlns="http://www.w3.org/2000/svg">
				<!-- Connecting lines -->
				<line x1="30" y1="70" x2="80" y2="26" stroke="oklch(0.40 0.01 270)" stroke-width="1" opacity="0.2" />
				<line x1="130" y1="70" x2="80" y2="26" stroke="oklch(0.40 0.01 270)" stroke-width="1" opacity="0.2" />
				<line x1="30" y1="70" x2="50" y2="80" stroke="oklch(0.40 0.01 270)" stroke-width="1" opacity="0.15" />
				<line x1="130" y1="70" x2="110" y2="80" stroke="oklch(0.40 0.01 270)" stroke-width="1" opacity="0.15" />
				<!-- Vertices -->
				<circle cx="80" cy="26" r="5" fill="oklch(0.50 0.08 250)" opacity="0.25" />
				<circle cx="30" cy="70" r="4" fill="oklch(0.45 0.06 250)" opacity="0.20" />
				<circle cx="130" cy="70" r="4" fill="oklch(0.45 0.06 250)" opacity="0.20" />
				<circle cx="50" cy="80" r="3" fill="oklch(0.40 0.04 250)" opacity="0.15" />
				<circle cx="110" cy="80" r="3" fill="oklch(0.40 0.04 250)" opacity="0.15" />
			</svg>
		</div>
	</div>
</section>

<!-- Source tabs + language filter -->
<section class="mx-auto max-w-6xl">
	<div class="mb-6 flex flex-wrap items-center justify-between gap-4">
		<!-- Source tabs -->
		<div class="flex items-center gap-1">
			{#each sourceTabs as tab}
				<button
					type="button"
					onclick={() => setSource(tab.value)}
					class="relative px-3 py-1.5 text-sm transition-colors duration-150
						{activeSource === tab.value
							? 'text-accent'
							: 'text-text-muted hover:text-text-secondary'}"
				>
					{tab.label}
					{#if activeSource === tab.value}
						<span class="absolute bottom-0 left-0 right-0 h-[2px] bg-accent"></span>
					{/if}
				</button>
			{/each}
		</div>

		<!-- Language filter -->
		<div class="flex items-center gap-2">
			{#if activeProtocol}
				<span class="flex items-center gap-1.5 rounded-full border border-accent/30 bg-accent/5 px-2.5 py-1 text-xs text-accent">
					<span class="h-1.5 w-1.5 rounded-full" style="background-color: {langColor(activeProtocol)}"></span>
					{ALL_LANGUAGES.find((l) => l.value === activeProtocol)?.label ?? activeProtocol}
					<a href="/" class="ml-0.5 text-text-muted hover:text-text-primary" title="Clear language filter">
						<svg class="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
							<path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
						</svg>
					</a>
				</span>
			{/if}

			<div class="relative">
				<input
					type="text"
					bind:value={searchQuery}
					onfocus={() => showDropdown = true}
					onblur={() => setTimeout(() => showDropdown = false, 200)}
					placeholder="Filter by language..."
					autocomplete="off"
					spellcheck="false"
					class="w-44 rounded-md border border-border bg-surface-0 px-3 py-1.5 text-xs text-text-primary placeholder:text-text-muted focus:border-accent focus:outline-none"
				/>

				{#if showDropdown}
					<ul class="absolute right-0 top-full z-50 mt-1 max-h-48 w-56 overflow-y-auto rounded-lg border border-border bg-surface-1 shadow-lg">
						{#each filtered as lang}
							<li>
								<button
									type="button"
									onmousedown={() => selectProtocol(lang.value)}
									class="flex w-full items-center gap-2 px-3 py-1.5 text-left text-xs transition-colors hover:bg-surface-2
										{activeProtocol === lang.value ? 'text-accent' : 'text-text-muted'}"
								>
									<span class="h-1.5 w-1.5 shrink-0 rounded-full" style="background-color: {langColor(lang.value)}"></span>
									{lang.label}
								</button>
							</li>
						{/each}
					</ul>
				{/if}
			</div>
		</div>
	</div>

	<!-- Trending section -->
	<div class="mb-10">
		<div class="mb-4 flex items-center gap-2">
			<svg class="h-4 w-4 text-warning" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
				<path stroke-linecap="round" stroke-linejoin="round" d="M2.25 18L9 11.25l4.306 4.307a11.95 11.95 0 015.814-5.519l2.74-1.22m0 0l-5.94-2.28m5.94 2.28l-2.28 5.941" />
			</svg>
			<h2 class="text-sm font-medium text-text-primary">Trending</h2>
			<span class="text-xs text-text-muted">by recent stars</span>
		</div>

		{#if data.trending.items.length === 0}
			<p class="py-12 text-center text-sm text-text-muted">
				{activeProtocol ? `No ${activeProtocol} repositories found.` : 'No repositories yet.'}
			</p>
		{:else}
			<div class="grid gap-4" style="grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));">
				{#each data.trending.items as repo (repo.did + '/' + repo.name)}
					<RepoCard {repo} />
				{/each}
			</div>
		{/if}
	</div>

	<!-- Recently updated section -->
	<div>
		<div class="mb-4 flex items-center gap-2">
			<svg class="h-4 w-4 text-accent" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
				<path stroke-linecap="round" stroke-linejoin="round" d="M12 6v6h4.5m4.5 0a9 9 0 11-18 0 9 9 0 0118 0z" />
			</svg>
			<h2 class="text-sm font-medium text-text-primary">Recently Updated</h2>
		</div>

		{#if data.recent.items.length === 0}
			<p class="py-12 text-center text-sm text-text-muted">
				No recently updated repositories.
			</p>
		{:else}
			<div class="grid gap-4" style="grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));">
				{#each data.recent.items as repo (repo.did + '/' + repo.name)}
					<RepoCard {repo} />
				{/each}
			</div>
		{/if}
	</div>
</section>
