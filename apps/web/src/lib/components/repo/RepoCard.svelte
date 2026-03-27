<script lang="ts">
	import type { Repo } from '$lib/api/repo.js';

	let { repo }: { repo: Repo } = $props();

	let displayName = $derived(repo.name || 'Untitled');
	let truncatedDescription = $derived(
		repo.description && repo.description.length > 140
			? repo.description.slice(0, 140) + '...'
			: repo.description
	);

	const protocolColors: Record<string, string> = {
		typescript: 'oklch(0.65 0.15 230)',
		python: 'oklch(0.65 0.15 90)',
		rust: 'oklch(0.60 0.18 30)',
		go: 'oklch(0.65 0.15 195)',
		protobuf: 'oklch(0.65 0.12 145)',
		graphql: 'oklch(0.55 0.20 330)',
		sql: 'oklch(0.65 0.10 260)',
		atproto_lexicon: 'oklch(0.65 0.15 250)',
		java: 'oklch(0.60 0.18 15)',
		ruby: 'oklch(0.55 0.20 10)',
		swift: 'oklch(0.65 0.18 30)',
		kotlin: 'oklch(0.55 0.20 280)',
		csharp: 'oklch(0.55 0.18 300)',
	};

	let protocolColor = $derived(protocolColors[repo.protocol] ?? 'oklch(0.50 0.02 260)');
</script>

<a
	href="/{repo.did}/{repo.name}"
	class="block rounded-lg border border-border bg-surface-1 p-4 transition-colors hover:border-accent"
>
	<div class="flex items-start justify-between gap-2">
		<h3 class="font-medium text-text-primary">{displayName}</h3>
		<span
			class="flex shrink-0 items-center gap-1.5 rounded-full bg-surface-2 px-2 py-0.5 font-mono text-xs"
		>
			<span class="h-2 w-2 rounded-full" style="background-color: {protocolColor}"></span>
			<span class="text-text-secondary">{repo.protocol}</span>
		</span>
	</div>

	{#if truncatedDescription}
		<p class="mt-1.5 text-sm text-text-secondary">{truncatedDescription}</p>
	{/if}

	<div class="mt-3 flex items-center gap-4 text-xs text-text-secondary">
		<span title="Stars">{repo.starCount} stars</span>
		{#if repo.openIssueCount > 0}
			<span title="Open issues">{repo.openIssueCount} issues</span>
		{/if}
		{#if repo.openMrCount > 0}
			<span title="Open merge requests">{repo.openMrCount} MRs</span>
		{/if}
	</div>
</a>
