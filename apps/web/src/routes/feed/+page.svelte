<script lang="ts">
	import { getAuth } from '$lib/stores/auth.svelte';
	import { timeAgo } from '$lib/utils/time.js';

	let { data } = $props();

	let auth = $derived(getAuth());

	const typeLabels: Record<string, string> = {
		refUpdate: 'pushed to',
		issue: 'opened an issue on',
		star: 'starred',
		pull: 'opened a merge request on',
		follow: 'started following',
	};

	const typeColors: Record<string, string> = {
		refUpdate: 'bg-compatible/15 text-compatible',
		issue: 'bg-accent/15 text-accent',
		star: 'bg-conflict/15 text-conflict',
		pull: 'bg-lens/15 text-lens',
		follow: 'bg-accent/15 text-accent',
	};
</script>

<svelte:head>
	<title>Feed - Cospan</title>
</svelte:head>

<section>
	<h1 class="mb-2 text-xl font-semibold text-text-primary">Activity Feed</h1>
	<p class="mt-1 text-sm text-text-secondary">
		Recent activity from users and repositories you follow.
	</p>

	{#if !auth.authenticated}
		<div class="mt-8 flex flex-col items-center gap-3 py-12 text-text-secondary">
			<svg class="h-10 w-10" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
				<path stroke-linecap="round" stroke-linejoin="round" d="M12 7.5h1.5m-1.5 3h1.5m-7.5 3h7.5m-7.5 3h7.5m3-9h3.375c.621 0 1.125.504 1.125 1.125V18a2.25 2.25 0 01-2.25 2.25M16.5 7.5V18a2.25 2.25 0 002.25 2.25M16.5 7.5V4.875c0-.621-.504-1.125-1.125-1.125H4.125C3.504 3.75 3 4.254 3 4.875V18a2.25 2.25 0 002.25 2.25h13.5M6 7.5h3v3H6v-3z" />
			</svg>
			<p class="text-sm">Sign in to see your personalized feed.</p>
		</div>
	{:else if data.items.length === 0}
		<div class="mt-8 flex flex-col items-center gap-3 py-12 text-text-secondary">
			<svg class="h-10 w-10" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
				<path stroke-linecap="round" stroke-linejoin="round" d="M12 7.5h1.5m-1.5 3h1.5m-7.5 3h7.5m-7.5 3h7.5m3-9h3.375c.621 0 1.125.504 1.125 1.125V18a2.25 2.25 0 01-2.25 2.25M16.5 7.5V18a2.25 2.25 0 002.25 2.25M16.5 7.5V4.875c0-.621-.504-1.125-1.125-1.125H4.125C3.504 3.75 3 4.254 3 4.875V18a2.25 2.25 0 002.25 2.25h13.5M6 7.5h3v3H6v-3z" />
			</svg>
			<p class="text-sm">Your feed is empty.</p>
			<p class="text-xs">Follow users and star repositories to see activity here.</p>
			<a
				href="/"
				class="mt-2 rounded-md border border-border bg-surface-1 px-4 py-2 text-xs text-text-secondary transition-colors hover:border-accent hover:text-text-primary"
			>
				Explore repositories
			</a>
		</div>
	{:else}
		<div class="mt-6 rounded-lg border border-border bg-surface-1">
			<ul class="divide-y divide-border">
				{#each data.items as item, i (i)}
					<li class="px-4 py-3">
						<div class="flex items-start gap-3">
							<span class="mt-0.5 shrink-0 rounded px-1.5 py-0.5 text-[10px] font-medium {typeColors[item.type] ?? 'bg-surface-2 text-text-secondary'}">
								{item.type}
							</span>
							<div class="min-w-0 flex-1">
								<p class="text-sm text-text-primary">
									<a
										href="/{item.actorDid}"
										class="font-medium transition-colors hover:text-accent"
									>
										{item.actorHandle ?? item.actorDid}
									</a>
									{' '}{typeLabels[item.type] ?? item.type}{' '}
									{#if item.repoDid && item.repoName}
										<a
											href="/{item.repoDid}/{item.repoName}"
											class="font-mono text-accent transition-colors hover:text-accent-hover"
										>
											{item.repoName}
										</a>
									{/if}
								</p>
								{#if item.subjectTitle}
									<p class="mt-0.5 truncate text-xs text-text-secondary">
										{item.subjectTitle}
									</p>
								{/if}
								{#if item.summary}
									<p class="mt-0.5 text-xs text-text-secondary">{item.summary}</p>
								{/if}
							</div>
							<time class="shrink-0 text-xs text-text-secondary" datetime={item.createdAt}>
								{timeAgo(item.createdAt)}
							</time>
						</div>
					</li>
				{/each}
			</ul>
		</div>
	{/if}
</section>
