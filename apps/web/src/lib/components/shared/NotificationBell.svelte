<script lang="ts">
	import { getAuth } from '$lib/stores/auth.svelte';
	import { getUnreadCount } from '$lib/api/notifications.js';
	import { onMount } from 'svelte';

	let auth = $derived(getAuth());
	let unreadCount = $state(0);

	onMount(() => {
		if (auth.authenticated) {
			loadUnreadCount();
			// Poll every 60 seconds
			const interval = setInterval(loadUnreadCount, 60_000);
			return () => clearInterval(interval);
		}
	});

	async function loadUnreadCount() {
		try {
			unreadCount = await getUnreadCount();
		} catch {
			// Silently fail; notification count is non-critical.
		}
	}
</script>

{#if auth.authenticated}
	<a
		href="/notifications"
		class="relative rounded-md border border-border bg-surface-1 p-1.5 text-text-secondary transition-colors hover:border-accent hover:text-text-primary"
		title="Notifications"
	>
		<svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
			<path stroke-linecap="round" stroke-linejoin="round" d="M14.857 17.082a23.848 23.848 0 005.454-1.31A8.967 8.967 0 0118 9.75v-.7V9A6 6 0 006 9v.75a8.967 8.967 0 01-2.312 6.022c1.733.64 3.56 1.085 5.455 1.31m5.714 0a24.255 24.255 0 01-5.714 0m5.714 0a3 3 0 11-5.714 0" />
		</svg>
		{#if unreadCount > 0}
			<span class="absolute -right-1 -top-1 flex h-4 min-w-4 items-center justify-center rounded-full bg-accent px-1 text-[10px] font-semibold text-surface-0">
				{unreadCount > 99 ? '99+' : unreadCount}
			</span>
		{/if}
	</a>
{/if}
