<script lang="ts">
	import { getAuth } from '$lib/stores/auth.svelte';

	let { subject, starCount = 0 }: { subject: string; starCount?: number } = $props();

	let auth = $derived(getAuth());
	let starred = $state(false);
	let count = $state(starCount);
	let toggling = $state(false);

	async function toggle() {
		if (!auth.authenticated || toggling) return;
		toggling = true;

		const wasStarred = starred;
		starred = !starred;
		count += starred ? 1 : -1;

		try {
			const resp = await fetch('/xrpc/dev.cospan.feed.star.toggle', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ subject, starred }),
			});
			if (!resp.ok) {
				starred = wasStarred;
				count += wasStarred ? 1 : -1;
			}
		} catch {
			starred = wasStarred;
			count += wasStarred ? 1 : -1;
		} finally {
			toggling = false;
		}
	}
</script>

<button
	onclick={toggle}
	disabled={!auth.authenticated || toggling}
	class="inline-flex items-center gap-1.5 rounded-md border px-3 py-1.5 text-xs transition-colors
		{starred
			? 'border-accent/40 bg-accent/10 text-accent'
			: 'border-border bg-surface-1 text-text-secondary hover:text-text-primary hover:border-accent/30'}
		disabled:opacity-50 disabled:cursor-default"
	title={auth.authenticated ? (starred ? 'Unstar' : 'Star') : 'Sign in to star'}
>
	<svg class="h-3.5 w-3.5" viewBox="0 0 24 24" fill={starred ? 'currentColor' : 'none'} stroke="currentColor" stroke-width="2">
		<path stroke-linecap="round" stroke-linejoin="round" d="M11.48 3.499a.562.562 0 011.04 0l2.125 5.111a.563.563 0 00.475.345l5.518.442c.499.04.701.663.321.988l-4.204 3.602a.563.563 0 00-.182.557l1.285 5.385a.562.562 0 01-.84.61l-4.725-2.885a.563.563 0 00-.586 0L6.982 20.54a.562.562 0 01-.84-.61l1.285-5.386a.562.562 0 00-.182-.557l-4.204-3.602a.563.563 0 01.321-.988l5.518-.442a.563.563 0 00.475-.345L11.48 3.5z" />
	</svg>
	{count}
</button>
