<script lang="ts">
	import RepoCard from '$lib/components/repo/RepoCard.svelte';
	import { ALL_LANGUAGES } from '$lib/data/languages';
	import { goto } from '$app/navigation';

	let { data } = $props();

	let activeProtocol = $derived(data.protocol);
	let searchQuery = $state('');
	let showDropdown = $state(false);

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
	<title>Explore Repositories - Cospan</title>
</svelte:head>

<section>
	<h1 class="mb-2 text-xl font-semibold text-text-primary">Explore</h1>
	<p class="mb-6 text-sm text-text-secondary">
		Discover repositories across protocols on the AT Protocol network.
	</p>

	<!-- Language filter -->
	<div class="mb-8">
		<div class="flex flex-wrap items-center gap-2">
			<a
				href="/"
				class="shrink-0 whitespace-nowrap rounded-full border px-3 py-1 text-xs font-medium transition-colors
					{!activeProtocol
						? 'border-accent bg-accent/10 text-accent'
						: 'border-border text-text-secondary hover:border-accent/30 hover:text-text-primary'}"
			>
				All {ALL_LANGUAGES.length} languages
			</a>

			{#if activeProtocol}
				<span class="flex items-center gap-1.5 rounded-full border border-accent bg-accent/10 px-3 py-1 text-xs font-medium text-accent">
					<span class="h-2 w-2 rounded-full" style="background-color: {langColor(activeProtocol)}"></span>
					{ALL_LANGUAGES.find((l) => l.value === activeProtocol)?.label ?? activeProtocol}
					<a href="/" class="ml-1 hover:text-text-primary">
						<svg class="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
							<path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
						</svg>
					</a>
				</span>
			{/if}

			<!-- Search input -->
			<div class="relative">
				<input
					type="text"
					bind:value={searchQuery}
					onfocus={() => showDropdown = true}
					onblur={() => setTimeout(() => showDropdown = false, 200)}
					placeholder="Filter by language..."
					autocomplete="off"
					spellcheck="false"
					class="w-48 rounded-full border border-border bg-surface-0 px-3 py-1 text-xs text-text-primary placeholder:text-text-secondary focus:border-accent focus:outline-none"
				/>

				{#if showDropdown}
					<ul class="absolute left-0 top-full z-50 mt-1 max-h-48 w-56 overflow-y-auto rounded-lg border border-border bg-surface-1 shadow-lg">
						{#each filtered as lang}
							<li>
								<button
									type="button"
									onmousedown={() => selectProtocol(lang.value)}
									class="flex w-full items-center gap-2 px-3 py-1.5 text-left text-xs transition-colors hover:bg-surface-2
										{activeProtocol === lang.value ? 'text-accent' : 'text-text-secondary'}"
								>
									<span class="h-2 w-2 shrink-0 rounded-full" style="background-color: {langColor(lang.value)}"></span>
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
			<svg class="h-4 w-4 text-conflict" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
				<path stroke-linecap="round" stroke-linejoin="round" d="M2.25 18L9 11.25l4.306 4.307a11.95 11.95 0 015.814-5.519l2.74-1.22m0 0l-5.94-2.28m5.94 2.28l-2.28 5.941" />
			</svg>
			<h2 class="text-lg font-medium text-text-primary">Trending</h2>
			<span class="text-xs text-text-secondary">by recent stars</span>
		</div>

		{#if data.trending.items.length === 0}
			<p class="py-8 text-center text-sm text-text-secondary">
				{activeProtocol ? `No ${activeProtocol} repositories found.` : 'No repositories yet.'}
			</p>
		{:else}
			<div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
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
			<h2 class="text-lg font-medium text-text-primary">Recently Updated</h2>
		</div>

		{#if data.recent.items.length === 0}
			<p class="py-8 text-center text-sm text-text-secondary">
				No recently updated repositories.
			</p>
		{:else}
			<div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
				{#each data.recent.items as repo (repo.did + '/' + repo.name)}
					<RepoCard {repo} />
				{/each}
			</div>
		{/if}
	</div>
</section>
