<script lang="ts">
	import { goto } from '$app/navigation';
	import Breadcrumb from '$lib/components/shared/Breadcrumb.svelte';

	let { data } = $props();

	let basePath = $derived(`/${data.did}/${data.repoName}`);

	let crumbs = $derived([
		{ label: data.did, href: `/${data.did}` },
		{ label: data.repoName, href: basePath },
		{ label: 'Compare' }
	]);

	let baseRef = $state(data.baseRef);
	let headRef = $state(data.headRef);

	let branches = $derived(data.refs.filter((r: { type: string }) => r.type === 'branch'));
	let tags = $derived(data.refs.filter((r: { type: string }) => r.type === 'tag'));
	let allRefs = $derived([...branches, ...tags]);

	let baseTarget = $derived(allRefs.find((r) => r.name === baseRef)?.target ?? null);
	let headTarget = $derived(allRefs.find((r) => r.name === headRef)?.target ?? null);

	let canCompare = $derived(baseRef && headRef && baseRef !== headRef);

	function updateUrl() {
		const params = new URLSearchParams();
		if (baseRef) params.set('base', baseRef);
		if (headRef) params.set('head', headRef);
		goto(`${basePath}/compare?${params.toString()}`, { replaceState: true });
	}

	function truncateHash(hash: string): string {
		return hash.slice(0, 10);
	}
</script>

<svelte:head>
	<title>Compare - {data.repoName} - Cospan</title>
</svelte:head>

<section>
	<Breadcrumb {crumbs} />

	<h1 class="mt-4 text-xl font-semibold text-text-primary">Compare refs</h1>
	<p class="mt-1 text-sm text-text-secondary">
		Select two branches or tags to compare their targets.
	</p>

	<!-- Ref selectors -->
	<div class="mt-6 flex flex-col gap-4 sm:flex-row sm:items-end">
		<div class="flex-1">
			<label for="base-ref" class="block text-xs font-medium text-text-secondary mb-1">Base</label>
			<select
				id="base-ref"
				bind:value={baseRef}
				onchange={updateUrl}
				class="w-full rounded-md border border-border bg-surface-1 px-3 py-2 font-mono text-sm text-text-primary focus:border-accent focus:outline-none"
			>
				<option value="">Select a ref...</option>
				{#if branches.length > 0}
					<optgroup label="Branches">
						{#each branches as ref (ref.name)}
							<option value={ref.name}>{ref.name}</option>
						{/each}
					</optgroup>
				{/if}
				{#if tags.length > 0}
					<optgroup label="Tags">
						{#each tags as ref (ref.name)}
							<option value={ref.name}>{ref.name}</option>
						{/each}
					</optgroup>
				{/if}
			</select>
		</div>

		<div class="flex items-center justify-center text-text-secondary">
			<svg class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
				<path stroke-linecap="round" stroke-linejoin="round" d="M7.5 21L3 16.5m0 0L7.5 12M3 16.5h13.5m0-13.5L21 7.5m0 0L16.5 12M21 7.5H7.5" />
			</svg>
		</div>

		<div class="flex-1">
			<label for="head-ref" class="block text-xs font-medium text-text-secondary mb-1">Head</label>
			<select
				id="head-ref"
				bind:value={headRef}
				onchange={updateUrl}
				class="w-full rounded-md border border-border bg-surface-1 px-3 py-2 font-mono text-sm text-text-primary focus:border-accent focus:outline-none"
			>
				<option value="">Select a ref...</option>
				{#if branches.length > 0}
					<optgroup label="Branches">
						{#each branches as ref (ref.name)}
							<option value={ref.name}>{ref.name}</option>
						{/each}
					</optgroup>
				{/if}
				{#if tags.length > 0}
					<optgroup label="Tags">
						{#each tags as ref (ref.name)}
							<option value={ref.name}>{ref.name}</option>
						{/each}
					</optgroup>
				{/if}
			</select>
		</div>
	</div>

	<!-- Comparison result -->
	{#if canCompare}
		<div class="mt-6 rounded-lg border border-border bg-surface-1 p-5">
			<h2 class="text-sm font-medium text-text-primary">
				Comparing <span class="font-mono text-accent">{baseRef}</span>
				with <span class="font-mono text-accent">{headRef}</span>
			</h2>

			<div class="mt-4 grid gap-4 sm:grid-cols-2">
				<div class="rounded-md border border-border bg-surface-0 p-3">
					<h3 class="text-xs font-medium text-text-secondary">Base target</h3>
					{#if baseTarget}
						<a
							href="{basePath}/commit/{baseTarget}"
							class="mt-1 block font-mono text-xs text-accent transition-colors hover:text-accent-hover"
						>
							{truncateHash(baseTarget)}
						</a>
					{:else}
						<span class="mt-1 block text-xs text-text-secondary">Not resolved</span>
					{/if}
				</div>

				<div class="rounded-md border border-border bg-surface-0 p-3">
					<h3 class="text-xs font-medium text-text-secondary">Head target</h3>
					{#if headTarget}
						<a
							href="{basePath}/commit/{headTarget}"
							class="mt-1 block font-mono text-xs text-accent transition-colors hover:text-accent-hover"
						>
							{truncateHash(headTarget)}
						</a>
					{:else}
						<span class="mt-1 block text-xs text-text-secondary">Not resolved</span>
					{/if}
				</div>
			</div>

			{#if baseTarget && headTarget && baseTarget === headTarget}
				<div class="mt-4 rounded-md bg-compatible/10 px-3 py-2 text-sm text-compatible">
					Both refs point to the same commit. No differences.
				</div>
			{:else if baseTarget && headTarget}
				<div class="mt-4 rounded-md border border-border bg-surface-0 p-4">
					<p class="text-sm text-text-secondary">
						Structural diff viewer will be displayed here once the panproto-wasm module
						is integrated. For now, the commit objects can be inspected individually.
					</p>
					<div class="mt-3 flex gap-2">
						<a
							href="{basePath}/commit/{baseTarget}"
							class="rounded-md border border-border bg-surface-1 px-3 py-1.5 text-xs text-text-secondary transition-colors hover:border-accent hover:text-text-primary"
						>
							View base commit
						</a>
						<a
							href="{basePath}/commit/{headTarget}"
							class="rounded-md border border-border bg-surface-1 px-3 py-1.5 text-xs text-text-secondary transition-colors hover:border-accent hover:text-text-primary"
						>
							View head commit
						</a>
					</div>
				</div>
			{/if}
		</div>
	{:else if data.refs.length === 0}
		<div class="mt-8 flex flex-col items-center gap-3 py-12 text-text-secondary">
			<p class="text-sm">No refs available for comparison.</p>
			<p class="text-xs">The node may be unreachable, or this repository has no branches.</p>
		</div>
	{:else if baseRef && headRef && baseRef === headRef}
		<div class="mt-6 rounded-md bg-conflict/10 px-3 py-2 text-sm text-conflict">
			Please select two different refs to compare.
		</div>
	{/if}
</section>
