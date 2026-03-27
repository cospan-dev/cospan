<script lang="ts">
	import type { NodeRef } from '$lib/api/node.js';

	let { refs, basePath }: { refs: NodeRef[]; basePath: string } = $props();

	let branches = $derived(refs.filter((r) => r.type === 'branch'));
	let tags = $derived(refs.filter((r) => r.type === 'tag'));

	function truncateHash(hash: string): string {
		return hash.slice(0, 10);
	}
</script>

<div class="rounded-lg border border-border bg-surface-1">
	{#if branches.length > 0}
		<div class="border-b border-border px-4 py-2">
			<h3 class="text-xs font-medium uppercase tracking-wide text-text-secondary">Branches</h3>
		</div>
		<ul class="divide-y divide-border">
			{#each branches as ref (ref.name)}
				<li>
					<a
						href="{basePath}/tree/{ref.name}"
						class="flex items-center justify-between gap-4 px-4 py-2.5 transition-colors hover:bg-surface-2"
					>
						<div class="flex items-center gap-2 min-w-0">
							<svg class="h-4 w-4 shrink-0 text-text-secondary" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
								<path stroke-linecap="round" stroke-linejoin="round" d="M13.19 8.688a4.5 4.5 0 011.242 7.244l-4.5 4.5a4.5 4.5 0 01-6.364-6.364l1.757-1.757m9.07-9.07a4.5 4.5 0 00-6.364 0l-4.5 4.5a4.5 4.5 0 006.364 6.364l1.757-1.757" />
							</svg>
							<span class="truncate text-sm text-text-primary">{ref.name}</span>
						</div>
						<code class="shrink-0 font-mono text-xs text-text-secondary">
							{truncateHash(ref.target)}
						</code>
					</a>
				</li>
			{/each}
		</ul>
	{/if}

	{#if tags.length > 0}
		<div class="border-b border-border px-4 py-2 {branches.length > 0 ? 'border-t' : ''}">
			<h3 class="text-xs font-medium uppercase tracking-wide text-text-secondary">Tags</h3>
		</div>
		<ul class="divide-y divide-border">
			{#each tags as ref (ref.name)}
				<li>
					<a
						href="{basePath}/tree/{ref.name}"
						class="flex items-center justify-between gap-4 px-4 py-2.5 transition-colors hover:bg-surface-2"
					>
						<div class="flex items-center gap-2 min-w-0">
							<svg class="h-4 w-4 shrink-0 text-text-secondary" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
								<path stroke-linecap="round" stroke-linejoin="round" d="M9.568 3H5.25A2.25 2.25 0 003 5.25v4.318c0 .597.237 1.17.659 1.591l9.581 9.581c.699.699 1.78.872 2.607.33a18.095 18.095 0 005.223-5.223c.542-.827.369-1.908-.33-2.607L11.16 3.66A2.25 2.25 0 009.568 3z" />
								<path stroke-linecap="round" stroke-linejoin="round" d="M6 6h.008v.008H6V6z" />
							</svg>
							<span class="truncate text-sm text-text-primary">{ref.name}</span>
						</div>
						<code class="shrink-0 font-mono text-xs text-text-secondary">
							{truncateHash(ref.target)}
						</code>
					</a>
				</li>
			{/each}
		</ul>
	{/if}

	{#if branches.length === 0 && tags.length === 0}
		<div class="flex flex-col items-center gap-3 py-12 text-text-secondary">
			<svg class="h-10 w-10" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
				<path stroke-linecap="round" stroke-linejoin="round" d="M2.25 12.75V12A2.25 2.25 0 014.5 9.75h15A2.25 2.25 0 0121.75 12v.75m-8.69-6.44l-2.12-2.12a1.5 1.5 0 00-1.061-.44H4.5A2.25 2.25 0 002.25 6v12a2.25 2.25 0 002.25 2.25h15A2.25 2.25 0 0021.75 18V9a2.25 2.25 0 00-2.25-2.25h-5.379a1.5 1.5 0 01-1.06-.44z" />
			</svg>
			<p class="text-sm">No refs found in this repository.</p>
		</div>
	{/if}
</div>
