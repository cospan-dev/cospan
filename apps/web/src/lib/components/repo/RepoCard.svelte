<script lang="ts">
	import type { Repo } from '$lib/api/repo.js';

	let { repo }: { repo: Repo } = $props();

	let displayName = $derived(repo.name || 'Untitled');

	// Known protocol hues; fallback uses a hash of the protocol name
	const protocolHues: Record<string, number> = {
		typescript: 230,
		javascript: 50,
		rust: 35,
		python: 95,
		go: 195,
		java: 15,
		kotlin: 280,
		swift: 30,
		ruby: 10,
		csharp: 300,
		protobuf: 145,
		graphql: 330,
		sql: 260,
		atproto_lexicon: 250,
	};

	function hashHue(value: string): number {
		let hash = 0;
		for (let i = 0; i < value.length; i++) {
			hash = value.charCodeAt(i) + ((hash << 5) - hash);
		}
		return Math.abs(hash % 360);
	}

	let hue = $derived(protocolHues[repo.protocol] ?? hashHue(repo.protocol));

	// Owner handle: extract from DID for display (placeholder until handle resolution)
	let ownerHandle = $derived(repo.did.startsWith('did:plc:') ? repo.did.slice(8, 20) + '...' : repo.did);
</script>

<a
	href="/{repo.did}/{repo.name}"
	class="group block rounded-lg border-l-2 bg-surface-0 p-4 transition-all duration-200 hover:bg-surface-1"
	style="
		border-left-color: oklch(0.45 0.12 {hue});
		background-color: oklch(0.14 0.015 {hue});
	"
	onmouseenter={(e) => {
		const el = e.currentTarget as HTMLElement;
		el.style.borderLeftColor = `oklch(0.55 0.14 ${hue})`;
		el.style.backgroundColor = `oklch(0.16 0.02 ${hue})`;
	}}
	onmouseleave={(e) => {
		const el = e.currentTarget as HTMLElement;
		el.style.borderLeftColor = `oklch(0.45 0.12 ${hue})`;
		el.style.backgroundColor = `oklch(0.14 0.015 ${hue})`;
	}}
>
	<!-- Top: owner handle + repo name -->
	<div class="flex items-baseline gap-1.5">
		<span class="text-xs text-text-muted">{ownerHandle}</span>
		<span class="text-xs text-text-muted">/</span>
		<span class="font-mono text-sm font-medium text-text-primary">{displayName}</span>
	</div>

	<!-- Middle: description (max 2 lines) -->
	{#if repo.description}
		<p class="mt-2 line-clamp-2 text-sm leading-relaxed text-text-secondary">
			{repo.description}
		</p>
	{/if}

	<!-- Bottom row: protocol, stats, source badge -->
	<div class="mt-3 flex items-center gap-3 text-xs">
		<span class="flex items-center gap-1.5">
			<span
				class="inline-block h-2 w-2 rounded-full"
				style="background-color: oklch(0.60 0.14 {hue})"
			></span>
			<span style="color: oklch(0.60 0.14 {hue})">{repo.protocol}</span>
		</span>

		{#if repo.starCount > 0}
			<span class="text-text-muted" title="Stars">
				<svg class="mr-0.5 inline-block h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
					<path stroke-linecap="round" stroke-linejoin="round" d="M11.48 3.499a.562.562 0 011.04 0l2.125 5.111a.563.563 0 00.475.345l5.518.442c.499.04.701.663.321.988l-4.204 3.602a.563.563 0 00-.182.557l1.285 5.385a.562.562 0 01-.84.61l-4.725-2.885a.562.562 0 00-.586 0L6.982 20.54a.562.562 0 01-.84-.61l1.285-5.386a.562.562 0 00-.182-.557l-4.204-3.602a.562.562 0 01.321-.988l5.518-.442a.563.563 0 00.475-.345L11.48 3.5z" />
				</svg>
				{repo.starCount}
			</span>
		{/if}

		{#if repo.openIssueCount > 0}
			<span class="text-text-muted" title="Open issues">
				{repo.openIssueCount} issues
			</span>
		{/if}
	</div>
</a>
