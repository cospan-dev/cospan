<script lang="ts">
	import { onMount } from 'svelte';
	import '../app.css';
	import Header from '$lib/components/layout/Header.svelte';
	import { initAuth, getAuth } from '$lib/stores/auth.svelte';

	let { children } = $props();
	let auth = $derived(getAuth());

	onMount(() => {
		initAuth();
	});
</script>

<div class="flex min-h-screen flex-col">
	<Header user={auth.authenticated ? { authenticated: true, did: auth.did ?? '', handle: auth.handle ?? '', displayName: auth.displayName, avatar: auth.avatar } : null} />
	<main class="mx-auto w-full max-w-6xl flex-1 px-4 py-6">
		{@render children()}
	</main>
</div>
