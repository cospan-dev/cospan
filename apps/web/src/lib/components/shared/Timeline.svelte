<script lang="ts">
	import StateBadge from '$lib/components/shared/StateBadge.svelte';
	import RichText from '$lib/components/shared/RichText.svelte';
	import { timeAgo } from '$lib/utils/time.js';

	interface TimelineEvent {
		kind: 'comment' | 'stateChange';
		[key: string]: unknown;
	}

	let { events = [] }: { events: TimelineEvent[] } = $props();
</script>

{#if events.length === 0}
	<p class="py-8 text-center text-sm text-text-secondary">No activity yet.</p>
{:else}
	<div class="space-y-4">
		{#each events as event}
			{#if event.kind === 'comment'}
				<div class="rounded-lg border border-border bg-surface-1">
					<div class="flex items-center gap-2 border-b border-border px-4 py-2">
						<span class="text-xs text-text-secondary">
							commented {timeAgo(String(event.createdAt ?? ''))}
						</span>
					</div>
					<div class="px-4 py-3 text-sm text-text-primary">
						<RichText text={String(event.body ?? '')} />
					</div>
				</div>
			{:else if event.kind === 'stateChange'}
				<div class="flex items-center gap-2 py-2 text-sm">
					<div class="h-px flex-1 bg-border"></div>
					<span class="flex items-center gap-2 text-text-secondary">
						{#if event.state === 'closed'}
							closed this
						{:else}
							reopened this
						{/if}
						<StateBadge state={String(event.state ?? 'open')} />
						<span class="text-xs">{timeAgo(String(event.createdAt ?? ''))}</span>
					</span>
					<div class="h-px flex-1 bg-border"></div>
				</div>
			{/if}
		{/each}
	</div>
{/if}
