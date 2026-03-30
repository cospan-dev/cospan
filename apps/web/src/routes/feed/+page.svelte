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

	const typeIcons: Record<string, string> = {
		refUpdate: 'M13 7l5 5m0 0l-5 5m5-5H6',
		issue: 'M12 9v3.75m9-.75a9 9 0 11-18 0 9 9 0 0118 0zm-9 3.75h.008v.008H12v-.008z',
		star: 'M11.48 3.499a.562.562 0 011.04 0l2.125 5.111a.563.563 0 00.475.345l5.518.442c.499.04.701.663.321.988l-4.204 3.602a.563.563 0 00-.182.557l1.285 5.385a.562.562 0 01-.84.61l-4.725-2.885a.563.563 0 00-.586 0L6.982 20.54a.562.562 0 01-.84-.61l1.285-5.386a.562.562 0 00-.182-.557l-4.204-3.602a.563.563 0 01.321-.988l5.518-.442a.563.563 0 00.475-.345L11.48 3.5z',
		pull: 'M7.5 21L3 16.5m0 0L7.5 12M3 16.5h13.5m0-13.5L21 7.5m0 0L16.5 12M21 7.5H7.5',
		follow: 'M19 7.5v3m3-3h-3m-2.25-4.125a3.375 3.375 0 11-6.75 0 3.375 3.375 0 016.75 0zM4 19.235v-.11a6.375 6.375 0 0112.75 0v.109A12.318 12.318 0 0110.374 21c-2.331 0-4.512-.645-6.374-1.766z',
	};

	const typeColors: Record<string, string> = {
		refUpdate: 'text-success',
		issue: 'text-accent',
		star: 'text-warning',
		pull: 'text-info',
		follow: 'text-accent',
	};
</script>

<svelte:head>
	<title>Feed · Cospan</title>
</svelte:head>

<section>
	<h1 class="mb-1 text-lg font-semibold text-ink">Activity Feed</h1>
	<p class="mb-6 text-[13px] text-caption">
		Recent activity from users and repositories you follow.
	</p>

	{#if !auth.authenticated}
		<div class="mt-8 flex flex-col items-center gap-3 py-16 text-text-muted">
			<svg class="h-10 w-10" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
				<path stroke-linecap="round" stroke-linejoin="round" d="M12 7.5h1.5m-1.5 3h1.5m-7.5 3h7.5m-7.5 3h7.5m3-9h3.375c.621 0 1.125.504 1.125 1.125V18a2.25 2.25 0 01-2.25 2.25M16.5 7.5V18a2.25 2.25 0 002.25 2.25M16.5 7.5V4.875c0-.621-.504-1.125-1.125-1.125H4.125C3.504 3.75 3 4.254 3 4.875V18a2.25 2.25 0 002.25 2.25h13.5M6 7.5h3v3H6v-3z" />
			</svg>
			<p class="text-sm">Sign in to see your personalized feed.</p>
		</div>
	{:else if data.items.length === 0}
		<div class="mt-8 flex flex-col items-center gap-3 py-16 text-text-muted">
			<svg class="h-10 w-10" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
				<path stroke-linecap="round" stroke-linejoin="round" d="M12 7.5h1.5m-1.5 3h1.5m-7.5 3h7.5m-7.5 3h7.5m3-9h3.375c.621 0 1.125.504 1.125 1.125V18a2.25 2.25 0 01-2.25 2.25M16.5 7.5V18a2.25 2.25 0 002.25 2.25M16.5 7.5V4.875c0-.621-.504-1.125-1.125-1.125H4.125C3.504 3.75 3 4.254 3 4.875V18a2.25 2.25 0 002.25 2.25h13.5M6 7.5h3v3H6v-3z" />
			</svg>
			<p class="text-sm">Your feed is empty.</p>
			<p class="text-xs">Follow users and star repositories to see activity here.</p>
			<a
				href="/"
				class="mt-2 rounded-md border border-border px-4 py-2 text-xs text-text-secondary transition-colors hover:border-accent hover:text-text-primary"
			>
				Explore repositories
			</a>
		</div>
	{:else}
		<div class="mt-2 space-y-0">
			{#each data.items as item, i (i)}
				<div class="relative flex gap-4 pb-6 last:pb-0">
					<!-- Timeline line -->
					{#if i < data.items.length - 1}
						<div class="absolute left-[13px] top-7 bottom-0 w-px bg-border"></div>
					{/if}

					<!-- Icon -->
					<div class="relative z-10 flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-surface-1 ring-2 ring-bg">
						<svg class="h-3.5 w-3.5 {typeColors[item.type] ?? 'text-text-muted'}" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
							<path stroke-linecap="round" stroke-linejoin="round" d={typeIcons[item.type] ?? 'M12 6v12m-3-2.818l.879.659c1.171.879 3.07.879 4.242 0 1.172-.879 1.172-2.303 0-3.182C13.536 12.219 12.768 12 12 12c-.725 0-1.45-.22-2.003-.659-1.106-.879-1.106-2.303 0-3.182s2.9-.879 4.006 0l.415.33M21 12a9 9 0 11-18 0 9 9 0 0118 0z'} />
						</svg>
					</div>

					<!-- Content -->
					<div class="min-w-0 flex-1 pt-0.5">
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
							<p class="mt-0.5 text-xs text-text-muted">{item.summary}</p>
						{/if}
						<time class="mt-1 block text-xs text-text-muted" datetime={item.createdAt}>
							{timeAgo(item.createdAt)}
						</time>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</section>
