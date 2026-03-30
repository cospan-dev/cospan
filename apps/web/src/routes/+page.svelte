<script lang="ts">
	import RepoCard from '$lib/components/repo/RepoCard.svelte';
	import { ALL_LANGUAGES } from '$lib/data/languages';
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';

	let { data } = $props();

	let searchQuery = $state('');
	let showDropdown = $state(false);

	// Multi-select protocols from URL
	let activeProtocols = $derived<string[]>(() => {
		const p = $page.url.searchParams.get('protocol');
		return p ? p.split(',').filter(Boolean) : [];
	});

	let filtered = $derived(
		searchQuery.trim()
			? ALL_LANGUAGES.filter((l) =>
					l.label.toLowerCase().includes(searchQuery.toLowerCase()) ||
					l.value.toLowerCase().includes(searchQuery.toLowerCase())
				).slice(0, 20)
			: ALL_LANGUAGES.slice(0, 20)
	);

	function toggleProtocol(value: string) {
		const current = activeProtocols();
		const next = current.includes(value)
			? current.filter((p) => p !== value)
			: [...current, value];
		showDropdown = false;
		searchQuery = '';
		const params = new URLSearchParams($page.url.searchParams);
		if (next.length === 0) {
			params.delete('protocol');
		} else {
			params.set('protocol', next.join(','));
		}
		const qs = params.toString();
		goto(qs ? `/?${qs}` : '/', { noScroll: true, replaceState: true });
	}

	function clearProtocols() {
		const params = new URLSearchParams($page.url.searchParams);
		params.delete('protocol');
		const qs = params.toString();
		goto(qs ? `/?${qs}` : '/', { noScroll: true, replaceState: true });
	}


	function langColor(value: string): string {
		let hash = 0;
		for (let i = 0; i < value.length; i++) hash = value.charCodeAt(i) + ((hash << 5) - hash);
		return `oklch(0.65 0.15 ${Math.abs(hash % 360)})`;
	}

	let totalRepos = $derived(data.trending.items.length + data.recent.items.length);
	let activeView = $state<'trending' | 'recent'>('trending');
	let activeItems = $derived(activeView === 'trending' ? data.trending.items : data.recent.items);
	let hasAnyRepos = $derived(data.trending.items.length > 0 || data.recent.items.length > 0);
</script>

<svelte:head>
	<title>Cospan · Schematic code hosting</title>
</svelte:head>

<!-- ═══ HERO ═══ -->
<section class="relative -mx-6 -mt-6 overflow-hidden border-b border-line/40">
	<!-- Ambient grid -->
	<div class="dot-grid pointer-events-none absolute inset-0 opacity-60"></div>

	<!-- Gradient atmosphere -->
	<div class="pointer-events-none absolute inset-0" style="background: radial-gradient(ellipse 80% 60% at 50% 0%, oklch(0.18 0.04 260 / 0.5), transparent 70%);"></div>

	<div class="relative mx-auto max-w-[1200px] px-6 pb-20 pt-24">
		<div class="max-w-2xl">
			<!-- Headline -->
			<h1 class="text-[clamp(2rem,5vw,3.25rem)] font-semibold leading-[1.1] tracking-tight text-ink">
				Code hosting with<br>
				<span class="text-caption">schematic version control.</span>
			</h1>

			<!-- Description -->
			<p class="mt-6 max-w-lg text-[15px] leading-relaxed text-caption">
				Structural diffs, schema-aware merges, and algebraic validation powered by panproto. Built on AT Protocol.
			</p>

		</div>

		<!-- Cospan diagram: abstract decorative element -->
		<div class="pointer-events-none absolute right-6 top-20 hidden opacity-[0.07] lg:block">
			<svg width="400" height="300" viewBox="0 0 400 300" fill="none">
				<!-- Large cospan: two morphisms meeting at apex -->
				<circle cx="200" cy="50" r="20" stroke="currentColor" stroke-width="1"/>
				<circle cx="60" cy="240" r="14" stroke="currentColor" stroke-width="0.8"/>
				<circle cx="340" cy="240" r="14" stroke="currentColor" stroke-width="0.8"/>
				<line x1="60" y1="240" x2="200" y2="50" stroke="currentColor" stroke-width="0.6"/>
				<line x1="340" y1="240" x2="200" y2="50" stroke="currentColor" stroke-width="0.6"/>

				<!-- Secondary structure: a pushout square -->
				<circle cx="130" cy="145" r="8" stroke="currentColor" stroke-width="0.5"/>
				<circle cx="270" cy="145" r="8" stroke="currentColor" stroke-width="0.5"/>
				<line x1="130" y1="145" x2="200" y2="50" stroke="currentColor" stroke-width="0.4" stroke-dasharray="4 4"/>
				<line x1="270" y1="145" x2="200" y2="50" stroke="currentColor" stroke-width="0.4" stroke-dasharray="4 4"/>
				<line x1="130" y1="145" x2="60" y2="240" stroke="currentColor" stroke-width="0.4" stroke-dasharray="4 4"/>
				<line x1="270" y1="145" x2="340" y2="240" stroke="currentColor" stroke-width="0.4" stroke-dasharray="4 4"/>
			</svg>
		</div>
	</div>
</section>

<!-- ═══ CONTROLS ═══ -->
<section>
	<div class="flex flex-wrap items-center justify-between gap-4 border-b border-line/40 py-4">
		<div class="flex items-center gap-3">
			<!-- View tabs: Trending / Recent -->
			<div class="flex items-center gap-0.5 rounded-lg bg-surface p-1">
				<button
					type="button"
					onclick={() => activeView = 'trending'}
					class="flex items-center gap-1.5 rounded-md px-3 py-1.5 text-[12px] font-medium transition-all
						{activeView === 'trending' ? 'bg-raised text-ink shadow-sm' : 'text-ghost hover:text-caption'}"
				>
					<svg class="h-3 w-3 {activeView === 'trending' ? 'text-warn' : ''}" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2.5">
						<path stroke-linecap="round" stroke-linejoin="round" d="M2.25 18L9 11.25l4.306 4.307a11.95 11.95 0 015.814-5.519l2.74-1.22m0 0l-5.94-2.28m5.94 2.28l-2.28 5.941" />
					</svg>
					Trending
				</button>
				<button
					type="button"
					onclick={() => activeView = 'recent'}
					class="flex items-center gap-1.5 rounded-md px-3 py-1.5 text-[12px] font-medium transition-all
						{activeView === 'recent' ? 'bg-raised text-ink shadow-sm' : 'text-ghost hover:text-caption'}"
				>
					<svg class="h-3 w-3 {activeView === 'recent' ? 'text-focus' : ''}" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2.5">
						<path stroke-linecap="round" stroke-linejoin="round" d="M12 6v6h4.5m4.5 0a9 9 0 11-18 0 9 9 0 0118 0z" />
					</svg>
					Recent
				</button>
			</div>
		</div>

		<!-- Language filter (multiselect) -->
		<div class="flex items-center gap-2">
			{#if activeProtocols().length > 1}
				<button
					type="button"
					onclick={clearProtocols}
					class="text-[11px] text-ghost hover:text-caption transition-colors"
				>
					Clear all
				</button>
			{/if}
			{#each activeProtocols() as proto}
				<button
					type="button"
					onclick={() => toggleProtocol(proto)}
					class="flex items-center gap-1.5 rounded-full border border-focus/30 bg-focus/5 px-2.5 py-1 text-[11px] font-medium text-focus-bright transition-colors hover:bg-focus/10"
				>
					<span class="h-1.5 w-1.5 rounded-full" style="background-color: {langColor(proto)}"></span>
					{ALL_LANGUAGES.find((l) => l.value === proto)?.label ?? proto}
					<span class="ml-0.5 text-ghost">×</span>
				</button>
			{/each}

			<div class="relative">
				<input
					type="text"
					bind:value={searchQuery}
					onfocus={() => showDropdown = true}
					onblur={() => setTimeout(() => showDropdown = false, 200)}
					onkeydown={(e) => {
						if (e.key === 'Enter' && filtered.length > 0) {
							e.preventDefault();
							toggleProtocol(filtered[0].value);
						}
					}}
					placeholder="Filter by language…"
					autocomplete="off"
					spellcheck="false"
					class="w-40 rounded-md border border-line bg-surface px-3 py-1.5 text-[12px] text-ink placeholder:text-ghost
						focus:border-focus/50 focus:outline-none transition-colors"
				/>
				{#if showDropdown && filtered.length > 0}
					<ul class="absolute right-0 top-full z-50 mt-1 max-h-52 w-56 overflow-y-auto rounded-lg border border-line bg-raised shadow-xl shadow-black/40">
						{#each filtered as lang}
							<li>
								<button
									type="button"
									onmousedown={() => toggleProtocol(lang.value)}
									class="flex w-full items-center gap-2 px-3 py-2 text-left text-[12px] transition-colors hover:bg-elevated
										{activeProtocols().includes(lang.value) ? 'text-focus-bright' : 'text-caption'}"
								>
									{#if activeProtocols().includes(lang.value)}
										<svg class="h-3 w-3 text-focus" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="3">
											<path stroke-linecap="round" stroke-linejoin="round" d="M4.5 12.75l6 6 9-13.5" />
										</svg>
									{:else}
										<span class="h-1.5 w-1.5 shrink-0 rounded-full" style="background-color: {langColor(lang.value)}"></span>
									{/if}
									{lang.label}
								</button>
							</li>
						{/each}
					</ul>
				{/if}
			</div>
		</div>
	</div>
</section>

<!-- ═══ REPOS ═══ -->
<section class="py-8">
	{#if activeItems.length === 0}
		<div class="flex flex-col items-center justify-center py-24 text-center">
			<div class="mb-4 text-ghost">
				<svg class="mx-auto h-10 w-10" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1">
					<path stroke-linecap="round" stroke-linejoin="round" d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4" />
				</svg>
			</div>
			<p class="text-sm text-caption">
				{activeProtocols().length > 0 ? `No repositories found for selected languages.` : 'No repositories yet.'}
			</p>
			<p class="mt-1 text-xs text-ghost">Repositories from Cospan and Tangled will appear here.</p>
		</div>
	{:else}
		<div class="grid gap-3" style="grid-template-columns: repeat(auto-fill, minmax(360px, 1fr));">
			{#each activeItems as repo (repo.did + '/' + repo.name)}
				<RepoCard {repo} />
			{/each}
		</div>
	{/if}
</section>
