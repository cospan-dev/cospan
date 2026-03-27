<script lang="ts">
	import { getAuth } from '$lib/stores/auth.svelte';

	let { repoDid, repoName, forkCount = 0 }: { repoDid: string; repoName: string; forkCount?: number } = $props();

	let auth = $derived(getAuth());
</script>

{#if auth.authenticated}
	<a
		href="/{repoDid}/{repoName}/fork"
		class="inline-flex items-center gap-1.5 rounded-md border border-border bg-surface-1 px-3 py-1.5 text-xs text-text-secondary transition-colors hover:border-accent/30 hover:text-text-primary"
		title="Fork this repository"
	>
		<svg class="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
			<path stroke-linecap="round" stroke-linejoin="round" d="M7.217 10.907a2.25 2.25 0 100 2.186m0-2.186c.18.324.283.696.283 1.093s-.103.77-.283 1.093m0-2.186l9.566-5.314m-9.566 7.5l9.566 5.314m0 0a2.25 2.25 0 103.935 2.186 2.25 2.25 0 00-3.935-2.186zm0-12.814a2.25 2.25 0 103.933-2.185 2.25 2.25 0 00-3.933 2.185z" />
		</svg>
		Fork
		{#if forkCount > 0}
			<span>{forkCount}</span>
		{/if}
	</a>
{/if}
