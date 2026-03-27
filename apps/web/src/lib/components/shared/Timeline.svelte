<script lang="ts">
	import type { TimelineEvent } from '$lib/api/issue.js';
	import StateBadge from '$lib/components/shared/StateBadge.svelte';
	import { timeAgo } from '$lib/utils/time.js';

	let { events }: { events: TimelineEvent[] } = $props();
</script>

{#if events.length === 0}
	<p class="py-8 text-center text-sm text-text-secondary">No activity yet.</p>
{:else}
	<div class="space-y-4">
		{#each events as event}
			{#if event.type === 'comment'}
				<div class="rounded-lg border border-border bg-surface-1">
					<div class="flex items-center gap-2 border-b border-border px-4 py-2">
						<span class="text-sm font-medium text-text-primary">
							{event.data.creatorHandle ?? event.data.creatorDid}
						</span>
						<span class="text-xs text-text-secondary">
							commented {timeAgo(event.data.createdAt)}
						</span>
					</div>
					<div class="px-4 py-3 text-sm text-text-primary whitespace-pre-wrap">
						{event.data.body}
					</div>
				</div>
			{:else if event.type === 'stateChange'}
				<div class="flex items-center gap-2 py-2 text-sm">
					<div class="h-px flex-1 bg-border"></div>
					<span class="flex items-center gap-2 text-text-secondary">
						<span class="font-medium text-text-primary">
							{event.data.actorHandle ?? event.data.actorDid}
						</span>
						{#if event.data.state === 'closed'}
							closed this
						{:else}
							reopened this
						{/if}
						<StateBadge state={event.data.state} />
						<span class="text-xs">{timeAgo(event.data.createdAt)}</span>
					</span>
					<div class="h-px flex-1 bg-border"></div>
				</div>
				{#if event.data.reason}
					<p class="ml-8 text-xs text-text-secondary">{event.data.reason}</p>
				{/if}
			{/if}
		{/each}
	</div>
{/if}
