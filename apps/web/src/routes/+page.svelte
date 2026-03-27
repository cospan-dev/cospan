<script lang="ts">
	import RepoCard from '$lib/components/repo/RepoCard.svelte';

	let { data } = $props();

	const protocols: { value: string; label: string; color: string }[] = [
		{ value: 'typescript', label: 'TypeScript', color: 'oklch(0.65 0.15 230)' },
		{ value: 'python', label: 'Python', color: 'oklch(0.65 0.15 90)' },
		{ value: 'rust', label: 'Rust', color: 'oklch(0.60 0.18 30)' },
		{ value: 'go', label: 'Go', color: 'oklch(0.65 0.15 195)' },
		{ value: 'protobuf', label: 'Protobuf', color: 'oklch(0.65 0.12 145)' },
		{ value: 'graphql', label: 'GraphQL', color: 'oklch(0.55 0.20 330)' },
		{ value: 'sql', label: 'SQL', color: 'oklch(0.65 0.10 260)' },
		{ value: 'atproto_lexicon', label: 'ATProto', color: 'oklch(0.65 0.15 250)' },
	];

	let activeProtocol = $derived(data.protocol);
</script>

<svelte:head>
	<title>Explore Repositories - Cospan</title>
</svelte:head>

<section>
	<h1 class="mb-2 text-2xl font-semibold text-text-primary">Explore</h1>
	<p class="mb-6 text-sm text-text-secondary">
		Discover repositories across protocols on the AT Protocol network.
	</p>

	<!-- Protocol filters -->
	<div class="mb-8 flex flex-wrap items-center gap-2">
		<a
			href="/"
			class="rounded-full border px-3 py-1 text-xs font-medium transition-colors
				{!activeProtocol
					? 'border-accent bg-accent/10 text-accent'
					: 'border-border text-text-secondary hover:border-accent/30 hover:text-text-primary'}"
		>
			All
		</a>
		{#each protocols as proto (proto.value)}
			<a
				href="/?protocol={proto.value}"
				class="rounded-full border px-3 py-1 text-xs font-medium transition-colors
					{activeProtocol === proto.value
						? 'border-accent bg-accent/10 text-accent'
						: 'border-border text-text-secondary hover:border-accent/30 hover:text-text-primary'}"
			>
				<span
					class="mr-1.5 inline-block h-2 w-2 rounded-full"
					style="background-color: {proto.color}"
				></span>
				{proto.label}
			</a>
		{/each}
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
				{activeProtocol ? `No ${activeProtocol} repositories found.` : 'No repositories yet. Be the first to create one.'}
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
