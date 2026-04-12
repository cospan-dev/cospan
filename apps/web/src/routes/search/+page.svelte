<script lang="ts">
	import { goto } from '$app/navigation';
	import RepoCard from '$lib/components/repo/RepoCard.svelte';
	import { debounce } from '$lib/utils/debounce.js';

	let { data } = $props();

	let inputValue = $state(data.query ?? '');
	let mode = $state(data.mode ?? 'repos');
	let anchor = $state(data.anchor ?? 'function');

	// Sync state when data changes (e.g., browser navigation)
	$effect(() => {
		inputValue = data.query ?? '';
		mode = data.mode ?? 'repos';
		anchor = data.anchor ?? 'function';
	});

	const debouncedSearch = debounce((q: string) => {
		if (q.trim()) {
			const params = new URLSearchParams({ q: q.trim(), mode });
			if (mode === 'structural') params.set('anchor', anchor);
			goto(`/search?${params}`, { keepFocus: true });
		}
	}, 300);

	function handleInput(event: Event) {
		const target = event.target as HTMLInputElement;
		inputValue = target.value;
		debouncedSearch(inputValue);
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter' && inputValue.trim()) {
			const params = new URLSearchParams({ q: inputValue.trim(), mode });
			if (mode === 'structural') params.set('anchor', anchor);
			goto(`/search?${params}`, { keepFocus: true });
		}
	}

	function switchMode(newMode: string) {
		mode = newMode;
		if (inputValue.trim()) {
			const params = new URLSearchParams({ q: inputValue.trim(), mode: newMode });
			if (newMode === 'structural') params.set('anchor', anchor);
			goto(`/search?${params}`, { keepFocus: true });
		}
	}

	function changeAnchor(event: Event) {
		anchor = (event.target as HTMLSelectElement).value;
		if (inputValue.trim()) {
			const params = new URLSearchParams({ q: inputValue.trim(), mode, anchor });
			goto(`/search?${params}`, { keepFocus: true });
		}
	}

	const anchors = [
		{ value: 'function', label: 'Functions' },
		{ value: 'record', label: 'Types / Records' },
		{ value: 'field', label: 'Fields' },
		{ value: 'refUpdate', label: 'Ref Updates' },
	];
</script>

<svelte:head>
	<title>{data.query ? `Search: ${data.query}` : 'Search'} · Cospan</title>
</svelte:head>

<section>
	<div class="mb-6">
		<h1 class="mb-1 text-lg font-semibold text-ink">Search</h1>
		<p class="mb-4 text-[13px] text-caption">Find repositories and schema elements across Cospan.</p>

		<!-- Mode tabs -->
		<div class="mb-3 flex items-center gap-1 border-b border-line">
			<button
				type="button"
				class="border-b-2 px-3 py-1.5 text-sm font-medium transition-colors {mode === 'repos'
					? 'border-focus text-ink'
					: 'border-transparent text-ghost hover:text-caption'}"
				onclick={() => switchMode('repos')}
			>
				Repositories
			</button>
			<button
				type="button"
				class="border-b-2 px-3 py-1.5 text-sm font-medium transition-colors {mode === 'structural'
					? 'border-focus text-ink'
					: 'border-transparent text-ghost hover:text-caption'}"
				onclick={() => switchMode('structural')}
			>
				Structural
			</button>
		</div>

		<div class="flex gap-2">
			<input
				type="text"
				value={inputValue}
				oninput={handleInput}
				onkeydown={handleKeydown}
				placeholder={mode === 'structural'
					? 'panproto expression (e.g., name == "validate")'
					: 'Search repositories...'}
				class="flex-1 rounded-md border border-line bg-surface px-3 py-2 text-[14px] text-ink placeholder:text-ghost focus:border-focus/50 focus:outline-none transition-colors"
			/>
			{#if mode === 'structural'}
				<select
					value={anchor}
					onchange={changeAnchor}
					class="rounded-md border border-line bg-surface px-2 py-2 text-[13px] text-ink focus:border-focus/50 focus:outline-none"
				>
					{#each anchors as a (a.value)}
						<option value={a.value}>{a.label}</option>
					{/each}
				</select>
			{/if}
		</div>

		{#if mode === 'structural'}
			<p class="mt-2 text-[11px] text-ghost">
				Uses panproto's expression language. Try: <code class="text-caption">name == "validate"</code>,
				<code class="text-caption">breaking_change_count > 0</code>,
				<code class="text-caption">protocol == "typescript"</code>
			</p>
		{/if}
	</div>

	<!-- Repository search results -->
	{#if mode === 'repos'}
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
	{/if}

	<!-- Structural search results -->
	{#if mode === 'structural'}
		{#if data.structuralResults === null}
			<div class="flex flex-col items-center gap-4 py-16 text-text-muted">
				<svg class="h-12 w-12" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
					<path stroke-linecap="round" stroke-linejoin="round" d="M3.75 6A2.25 2.25 0 016 3.75h2.25A2.25 2.25 0 0110.5 6v2.25a2.25 2.25 0 01-2.25 2.25H6a2.25 2.25 0 01-2.25-2.25V6zM3.75 15.75A2.25 2.25 0 016 13.5h2.25a2.25 2.25 0 012.25 2.25V18a2.25 2.25 0 01-2.25 2.25H6A2.25 2.25 0 013.75 18v-2.25zM13.5 6a2.25 2.25 0 012.25-2.25H18A2.25 2.25 0 0120.25 6v2.25A2.25 2.25 0 0118 10.5h-2.25a2.25 2.25 0 01-2.25-2.25V6zM13.5 15.75a2.25 2.25 0 012.25-2.25H18a2.25 2.25 0 012.25 2.25V18A2.25 2.25 0 0118 20.25h-2.25A2.25 2.25 0 0113.5 18v-2.25z" />
				</svg>
				<p class="text-sm">Enter a panproto expression to search schema elements.</p>
			</div>
		{:else if data.structuralResults.results.length === 0}
			<div class="flex flex-col items-center gap-4 py-16 text-text-muted">
				<svg class="h-12 w-12" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
					<path stroke-linecap="round" stroke-linejoin="round" d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4" />
				</svg>
				<p class="text-sm">No schema elements matched the expression.</p>
			</div>
		{:else}
			<div class="mb-4 flex items-center gap-3 text-sm text-text-muted">
				<span>{data.structuralResults.total} {data.structuralResults.total === 1 ? 'result' : 'results'}</span>
				<span class="text-text-muted">&middot;</span>
				<span>anchor: <code class="rounded bg-surface-2 px-1 py-0.5 text-xs text-accent">{data.structuralResults.anchor}</code></span>
				<span class="text-text-muted">&middot;</span>
				<span>expression: <code class="rounded bg-surface-2 px-1 py-0.5 text-xs text-caption">{data.structuralResults.expression}</code></span>
			</div>

			<div class="space-y-2">
				{#each data.structuralResults.results as result, i (i)}
					{@const repoDid = result._repo_did as string ?? ''}
					{@const repoName = result._repo_name as string ?? ''}
					<div class="rounded-lg border border-border bg-surface-1 px-4 py-3">
						<div class="mb-1 flex items-center gap-2">
							<a
								href="/{repoDid}/{repoName}"
								class="font-mono text-xs text-accent hover:underline"
							>
								{repoDid}/{repoName}
							</a>
						</div>
						<div class="flex flex-wrap gap-x-4 gap-y-1 text-[12px] text-text-secondary">
							{#each Object.entries(result).filter(([k]) => !k.startsWith('_')) as [key, value] (key)}
								<span>
									<span class="text-text-muted">{key}:</span>
									<span class="font-mono text-text-primary">
										{typeof value === 'object' ? JSON.stringify(value) : String(value)}
									</span>
								</span>
							{/each}
						</div>
					</div>
				{/each}
			</div>
		{/if}
	{/if}
</section>
