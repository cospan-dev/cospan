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

<div class="flex min-h-screen flex-col bg-void">
	<Header user={auth.authenticated ? { authenticated: true, did: auth.did ?? '', handle: auth.handle ?? '', displayName: auth.displayName, avatar: auth.avatar } : null} />
	<main class="mx-auto w-full max-w-[1200px] flex-1 px-6 py-6">
		{@render children()}
	</main>
	<footer class="border-t border-line/50 py-8">
		<div class="mx-auto max-w-[1200px] px-6 flex items-center justify-between text-xs text-ghost">
			<span>cospan — schema-first code hosting</span>
			<span>built on <a href="https://atproto.com" class="text-caption hover:text-ink transition-colors">AT Protocol</a></span>
		</div>
	</footer>
</div>
