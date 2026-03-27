<script lang="ts">
	import { page } from '$app/stores';
</script>

<svelte:head>
	<title>Error - Cospan</title>
</svelte:head>

<div class="flex flex-col items-center justify-center py-24">
	<h1 class="text-4xl font-bold text-text-primary">{$page.status}</h1>
	<p class="mt-2 text-text-secondary">{$page.error?.message ?? 'Something went wrong.'}</p>
	<a href="/" class="mt-6 text-sm text-accent hover:text-accent-hover transition-colors">
		Back to home
	</a>
</div>
