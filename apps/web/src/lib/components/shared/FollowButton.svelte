<script lang="ts">
	import { getAuth } from '$lib/stores/auth.svelte';

	let { subjectDid, followerCount = 0 }: { subjectDid: string; followerCount?: number } = $props();

	let auth = $derived(getAuth());
	let following = $state(false);
	let count = $state(followerCount);
	let toggling = $state(false);

	// Do not show follow button for your own profile
	let isSelf = $derived(auth.authenticated && auth.did === subjectDid);

	async function toggle() {
		if (!auth.authenticated || toggling || isSelf) return;
		toggling = true;

		const wasFollowing = following;
		following = !following;
		count += following ? 1 : -1;

		try {
			const resp = await fetch('/xrpc/dev.cospan.graph.follow.toggle', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ subject: subjectDid, following }),
			});
			if (!resp.ok) {
				following = wasFollowing;
				count += wasFollowing ? 1 : -1;
			}
		} catch {
			following = wasFollowing;
			count += wasFollowing ? 1 : -1;
		} finally {
			toggling = false;
		}
	}
</script>

{#if !isSelf}
	<button
		onclick={toggle}
		disabled={!auth.authenticated || toggling}
		class="inline-flex items-center gap-1.5 rounded-md border px-3 py-1.5 text-xs transition-colors
			{following
				? 'border-accent/40 bg-accent/10 text-accent'
				: 'border-border bg-surface-1 text-text-secondary hover:text-text-primary hover:border-accent/30'}
			disabled:opacity-50 disabled:cursor-default"
		title={auth.authenticated ? (following ? 'Unfollow' : 'Follow') : 'Sign in to follow'}
	>
		<svg class="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
			{#if following}
				<path stroke-linecap="round" stroke-linejoin="round" d="M15 19.128a9.38 9.38 0 002.625.372 9.337 9.337 0 004.121-.952 4.125 4.125 0 00-7.533-2.493M15 19.128v-.003c0-1.113-.285-2.16-.786-3.07M15 19.128v.106A12.318 12.318 0 018.624 21c-2.331 0-4.512-.645-6.374-1.766l-.001-.109a6.375 6.375 0 0111.964-3.07M12 6.375a3.375 3.375 0 11-6.75 0 3.375 3.375 0 016.75 0zm8.25 2.25h-2.25" />
			{:else}
				<path stroke-linecap="round" stroke-linejoin="round" d="M19 7.5v3m0 0v3m0-3h3m-3 0h-3m-2.25-4.125a3.375 3.375 0 11-6.75 0 3.375 3.375 0 016.75 0zM4 19.235v-.11a6.375 6.375 0 0112.75 0v.109A12.318 12.318 0 018.624 21c-2.331 0-4.512-.645-6.374-1.766z" />
			{/if}
		</svg>
		{following ? 'Following' : 'Follow'}
		{#if count > 0}
			<span class="text-text-secondary">{count}</span>
		{/if}
	</button>
{/if}
