<script lang="ts">
	import type { RefUpdateView } from '$lib/generated/views.js';

	let { refUpdates }: { refUpdates: RefUpdateView[] } = $props();

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
</script>

{#if refUpdates.length === 0}
	<p class="py-8 text-center text-sm text-text-secondary">No commits yet.</p>
{:else}
	<ul class="divide-y divide-border">
		{#each refUpdates as update (update.rkey)}
			<li class="flex items-center justify-between gap-4 py-3">
				<div class="min-w-0 flex-1">
					<div class="flex items-center gap-2">
						<span class="rounded bg-surface-2 px-1.5 py-0.5 font-mono text-xs text-accent">
							{(update.refName ?? '').replace('refs/heads/', '') || 'unknown'}
						</span>
						<code class="font-mono text-xs text-text-secondary">
							{truncateHash(update.newTarget)}
						</code>
					</div>
					{#if update.committerDid}
						<p class="mt-0.5 text-sm text-text-secondary">
							{update.committerDid}
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
			</li>
		{/each}
	</ul>
{/if}
