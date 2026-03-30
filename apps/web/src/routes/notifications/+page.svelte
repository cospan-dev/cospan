<script lang="ts">
	import { getAuth } from '$lib/stores/auth.svelte';
	import { listNotifications, markAllRead } from '$lib/api/notifications.js';
	import type { Notification } from '$lib/api/notifications.js';
	import { timeAgo } from '$lib/utils/time.js';
	import { onMount } from 'svelte';

	let auth = $derived(getAuth());
	let notifications = $state<Notification[]>([]);
	let loading = $state(true);

	onMount(async () => {
		try {
			const result = await listNotifications({ limit: 50 });
			notifications = result.items;
		} catch {
			// Failed to load notifications.
		} finally {
			loading = false;
		}
	});

	async function handleMarkAllRead() {
		try {
			await markAllRead();
			notifications = notifications.map((n) => ({ ...n, isRead: true }));
		} catch {
			// Mark-all-read failed silently.
		}
	}

	const typeIcons: Record<string, string> = {
		star: 'Star',
		follow: 'Follow',
		issue: 'Issue',
		pull: 'MR',
		comment: 'Comment',
		mention: 'Mention',
		refUpdate: 'Push',
	};

	const typeColors: Record<string, string> = {
		star: 'text-conflict',
		follow: 'text-accent',
		issue: 'text-compatible',
		pull: 'text-lens',
		comment: 'text-text-secondary',
		mention: 'text-accent',
		refUpdate: 'text-compatible',
	};
</script>

<svelte:head>
	<title>Notifications · Cospan</title>
</svelte:head>

<section>
	<div class="flex items-center justify-between">
		<div>
			<h1 class="text-xl font-semibold text-text-primary">Notifications</h1>
			<p class="mt-1 text-sm text-text-muted">Updates from repositories and users you follow.</p>
		</div>
		{#if notifications.some((n) => !n.isRead)}
			<button
				onclick={handleMarkAllRead}
				class="rounded-md border border-border bg-surface-1 px-3 py-1.5 text-xs text-text-secondary transition-colors hover:border-accent hover:text-text-primary"
			>
				Mark all as read
			</button>
		{/if}
	</div>

	{#if !auth.authenticated}
		<div class="mt-8 flex flex-col items-center gap-3 py-12 text-text-secondary">
			<svg class="h-10 w-10" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
				<path stroke-linecap="round" stroke-linejoin="round" d="M14.857 17.082a23.848 23.848 0 005.454-1.31A8.967 8.967 0 0118 9.75v-.7V9A6 6 0 006 9v.75a8.967 8.967 0 01-2.312 6.022c1.733.64 3.56 1.085 5.455 1.31m5.714 0a24.255 24.255 0 01-5.714 0m5.714 0a3 3 0 11-5.714 0" />
			</svg>
			<p class="text-sm">Sign in to view your notifications.</p>
		</div>
	{:else if loading}
		<div class="mt-6 space-y-3">
			{#each Array(5) as _}
				<div class="h-16 animate-pulse rounded-lg bg-surface-1"></div>
			{/each}
		</div>
	{:else if notifications.length === 0}
		<div class="mt-8 flex flex-col items-center gap-3 py-12 text-text-secondary">
			<svg class="h-10 w-10" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
				<path stroke-linecap="round" stroke-linejoin="round" d="M14.857 17.082a23.848 23.848 0 005.454-1.31A8.967 8.967 0 0118 9.75v-.7V9A6 6 0 006 9v.75a8.967 8.967 0 01-2.312 6.022c1.733.64 3.56 1.085 5.455 1.31m5.714 0a24.255 24.255 0 01-5.714 0m5.714 0a3 3 0 11-5.714 0" />
			</svg>
			<p class="text-sm">No notifications yet.</p>
			<p class="text-xs">You will be notified when someone stars your repos, comments on issues, or mentions you.</p>
		</div>
	{:else}
		<div class="mt-4 rounded-lg border border-border bg-surface-1">
			<ul class="divide-y divide-border">
				{#each notifications as notif (notif.rkey)}
					<li class="flex items-start gap-3 px-4 py-3 {notif.isRead ? 'opacity-60' : ''}">
						<span class="mt-0.5 shrink-0 rounded bg-surface-2 px-1.5 py-0.5 text-[10px] font-medium {typeColors[notif.type] ?? 'text-text-secondary'}">
							{typeIcons[notif.type] ?? notif.type}
						</span>
						<div class="min-w-0 flex-1">
							<p class="text-sm text-text-primary">
								<span class="font-medium">{notif.actorHandle ?? notif.actorDid}</span>
								{' '}{notif.reason}
							</p>
							{#if notif.subjectTitle}
								<p class="mt-0.5 truncate text-xs text-text-secondary">{notif.subjectTitle}</p>
							{/if}
						</div>
						<time class="shrink-0 text-xs text-text-secondary" datetime={notif.createdAt}>
							{timeAgo(notif.createdAt)}
						</time>
						{#if !notif.isRead}
							<span class="mt-1.5 h-2 w-2 shrink-0 rounded-full bg-accent"></span>
						{/if}
					</li>
				{/each}
			</ul>
		</div>
	{/if}
</section>
