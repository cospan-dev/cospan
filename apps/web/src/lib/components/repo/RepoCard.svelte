<script lang="ts">
	import type { Repo } from '$lib/api/repo.js';

	let { repo }: { repo: Repo } = $props();

	let displayName = $derived(repo.name || 'Untitled');

	const hueMap: Record<string, number> = {
		typescript: 230, javascript: 50, rust: 25, python: 100,
		go: 175, java: 10, kotlin: 275, swift: 15, ruby: 350,
		csharp: 290, protobuf: 145, graphql: 320, sql: 210, git: 15,
	};

	function hashHue(v: string): number {
		let h = 0;
		for (let i = 0; i < v.length; i++) h = v.charCodeAt(i) + ((h << 5) - h);
		return Math.abs(h % 360);
	}

	let hue = $derived(hueMap[repo.protocol] ?? hashHue(repo.protocol));
	let ownerHandle = $derived(
		repo.did.startsWith('did:plc:') ? repo.did.slice(8, 18) + '…' : repo.did
	);
	let isTangled = $derived(repo.source === 'tangled');
</script>

<a
	href="/{repo.did}/{repo.name}"
	class="group relative flex flex-col rounded-lg border border-line/60 bg-ground p-4 transition-all duration-200 hover:border-line-bright hover:bg-surface"
>
	<!-- Protocol accent: left edge glow -->
	<div
		class="absolute left-0 top-3 bottom-3 w-[2px] rounded-full transition-opacity duration-200 group-hover:opacity-100"
		style="background: oklch(0.55 0.16 {hue}); opacity: 0.5;"
	></div>

	<!-- Header -->
	<div class="flex items-center gap-1.5 pl-2.5">
		<span class="text-[11px] text-ghost">{ownerHandle}</span>
		<span class="text-[11px] text-ghost/40">/</span>
		<span class="font-mono text-[13px] font-medium text-ink">{displayName}</span>
	</div>

	<!-- Description -->
	{#if repo.description}
		<p class="mt-2 line-clamp-2 pl-2.5 text-[13px] leading-relaxed text-caption">
			{repo.description}
		</p>
	{/if}

	<!-- Footer: metadata row -->
	<div class="mt-auto flex items-center gap-3 pt-3 pl-2.5 text-[11px] font-medium">
		<!-- Protocol dot + name -->
		<span class="flex items-center gap-1.5">
			<span class="h-[6px] w-[6px] rounded-full" style="background: oklch(0.60 0.16 {hue})"></span>
			<span class="text-ghost" style="color: oklch(0.55 0.10 {hue})">{repo.protocol}</span>
		</span>

		{#if repo.starCount > 0}
			<span class="flex items-center gap-1 text-ghost" title="Stars">
				★ {repo.starCount}
			</span>
		{/if}

		{#if repo.openIssueCount > 0}
			<span class="text-ghost" title="Open issues">
				{repo.openIssueCount} issues
			</span>
		{/if}

		<!-- Source badge (right-aligned) -->
		{#if isTangled}
			<span class="ml-auto rounded-full border border-info/20 bg-info/5 px-2 py-0.5 text-[10px] font-semibold text-info">
				tangled
			</span>
		{/if}
	</div>
</a>
