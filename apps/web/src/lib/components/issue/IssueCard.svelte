<script lang="ts">
	import type { IssueView } from '$lib/generated/views.js';
	import StateBadge from '$lib/components/shared/StateBadge.svelte';
	import { timeAgo } from '$lib/utils/time.js';

	let { issue, basePath }: { issue: IssueView; basePath: string } = $props();
</script>

<a
	href="{basePath}/issues/{issue.rkey}"
	class="block rounded-lg border border-border bg-surface-1 p-4 transition-all hover:border-border-hover"
>
	<div class="flex items-start gap-3">
		<div class="mt-0.5">
			<StateBadge state={issue.state} />
		</div>
		<div class="min-w-0 flex-1">
			<h3 class="font-semibold text-text-primary">{issue.title}</h3>
			<div class="mt-1.5 flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-text-muted">
				<span>#{issue.rkey.slice(0, 7)}</span>
				{#if issue.commentCount > 0}
					<span class="flex items-center gap-1">
						<svg class="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
							<path stroke-linecap="round" stroke-linejoin="round" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
						</svg>
						{issue.commentCount}
					</span>
				{/if}
				<span>{timeAgo(issue.createdAt)}</span>
			</div>
		</div>
	</div>
</a>
