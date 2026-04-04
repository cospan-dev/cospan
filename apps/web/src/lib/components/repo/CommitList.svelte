<script lang="ts">
	import { onMount } from 'svelte';
	import type { RefUpdateView } from '$lib/generated/views.js';
	import { resolveHandle } from '$lib/api/handle.js';

	let {
		refUpdates,
		commitUrlBase = ''
	}: {
		refUpdates: RefUpdateView[];
		commitUrlBase?: string;
	} = $props();

	let handles = $state<Record<string, string>>({});

	onMount(async () => {
		const dids = [...new Set(refUpdates.map(u => u.committerDid).filter(Boolean))];
		const resolved: Record<string, string> = {};
		await Promise.allSettled(
			dids.map(async (did) => { resolved[did] = await resolveHandle(did); })
		);
		handles = resolved;
	});

	function displayName(did: string): string {
		return handles[did] || (did.startsWith('did:plc:') ? did.slice(8, 18) + '\u2026' : did);
	}

	function truncateHash(hash: string): string {
		return hash.slice(0, 8);
	}

	function formatTimestamp(iso: string): string {
		const date = new Date(iso);
		return date.toLocaleDateString('en-US', {
			month: 'short',
			day: 'numeric',
			year: 'numeric'
		});
	}

	function commitUrl(hash: string): string | undefined {
		if (!commitUrlBase || !hash) return undefined;
		return `${commitUrlBase}/${hash}`;
	}

	let isExternal = $derived(commitUrlBase.startsWith('http'));
</script>

{#if refUpdates.length === 0}
	<p class="py-8 text-center text-sm text-text-secondary">No commits yet.</p>
{:else}
	<ul class="divide-y divide-border">
		{#each refUpdates as update (update.rkey)}
			{@const url = commitUrl(update.newTarget)}
			<li>
				{#if url}
					<a
						href={url}
						class="flex items-center justify-between gap-4 py-3 -mx-2 px-2 rounded transition-colors hover:bg-surface-2"
						target={isExternal ? '_blank' : undefined}
						rel={isExternal ? 'noopener noreferrer' : undefined}
					>
						{@render commitContent(update)}
					</a>
				{:else}
					<div class="flex items-center justify-between gap-4 py-3">
						{@render commitContent(update)}
					</div>
				{/if}
			</li>
		{/each}
	</ul>
{/if}

{#snippet commitContent(update: RefUpdateView)}
	<div class="min-w-0 flex-1">
		<div class="flex items-center gap-2">
			<span class="rounded bg-surface-2 px-1.5 py-0.5 font-mono text-xs text-accent">
				{(update.refName ?? '').replace('refs/heads/', '').replace('refs/tags/', '') || 'unknown'}
			</span>
			<code class="font-mono text-xs text-text-secondary">
				{truncateHash(update.newTarget)}
			</code>
		</div>
		{#if update.committerDid}
			<p class="mt-0.5 text-xs text-text-secondary">
				{displayName(update.committerDid)}
			</p>
		{/if}
	</div>
	<div class="flex shrink-0 items-center gap-3 text-xs text-text-secondary">
		{#if update.breakingChangeCount > 0}
			<span class="font-medium text-breaking">
				{update.breakingChangeCount} breaking
			</span>
		{/if}
		{#if update.lensQuality != null && typeof update.lensQuality === 'number'}
			<span title="Lens quality">
				lens {(update.lensQuality * 100).toFixed(0)}%
			</span>
		{/if}
		<time datetime={update.createdAt}>
			{formatTimestamp(update.createdAt)}
		</time>
	</div>
{/snippet}
