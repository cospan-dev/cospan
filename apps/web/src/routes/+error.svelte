<script lang="ts">
	import { page } from '$app/stores';
</script>

<svelte:head>
	<title>{$page.status} - Cospan</title>
</svelte:head>

<div class="flex flex-col items-center justify-center py-24">
	<svg class="mb-6 h-16 w-16 text-text-secondary" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1">
		{#if $page.status === 404}
			<path stroke-linecap="round" stroke-linejoin="round" d="M21 21l-5.197-5.197m0 0A7.5 7.5 0 105.196 5.196a7.5 7.5 0 0010.607 10.607z" />
		{:else if $page.status >= 500}
			<path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126zM12 15.75h.007v.008H12v-.008z" />
		{:else}
			<path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m9-.75a9 9 0 11-18 0 9 9 0 0118 0zm-9 3.75h.008v.008H12v-.008z" />
		{/if}
	</svg>
	<h1 class="text-4xl font-bold text-text-primary">{$page.status}</h1>
	<p class="mt-2 text-sm text-text-secondary">{$page.error?.message ?? 'Something went wrong.'}</p>
	<div class="mt-6 flex items-center gap-3">
		<button
			onclick={() => history.back()}
			class="rounded-md border border-border bg-surface-1 px-4 py-2 text-sm text-text-secondary transition-colors hover:border-accent hover:text-text-primary"
		>
			Go back
		</button>
		<a href="/" class="rounded-md bg-accent px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-accent-hover">
			Back to home
		</a>
	</div>
</div>
