<script lang="ts">
	import { goto } from '$app/navigation';
	import { debounce } from '$lib/utils/debounce.js';
	import UserMenu from '$lib/components/auth/UserMenu.svelte';
	import NotificationBell from '$lib/components/shared/NotificationBell.svelte';

	let { user }: { user?: { authenticated: boolean; did: string; handle: string; displayName?: string; avatar?: string } | null } = $props();

	let searchValue = $state('');
	let searchInputRef = $state<HTMLInputElement | null>(null);

	const submitSearch = debounce((q: string) => {
		if (q.trim()) {
			goto(`/search?q=${encodeURIComponent(q.trim())}`);
		}
	}, 300);

	function handleSearchKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter' && searchValue.trim()) {
			goto(`/search?q=${encodeURIComponent(searchValue.trim())}`);
		}
	}

	function handleGlobalKeydown(event: KeyboardEvent) {
		// Do not capture when typing in an input, textarea, or contenteditable
		const target = event.target as HTMLElement;
		if (
			target.tagName === 'INPUT' ||
			target.tagName === 'TEXTAREA' ||
			target.isContentEditable
		) {
			return;
		}

		// "/" to focus search
		if (event.key === '/') {
			event.preventDefault();
			searchInputRef?.focus();
		}
	}
</script>

<svelte:window onkeydown={handleGlobalKeydown} />

<header class="border-b border-border bg-surface-1">
	<nav class="mx-auto flex h-12 max-w-6xl items-center justify-between px-4">
		<div class="flex items-center gap-6">
			<a href="/" class="flex items-center gap-2 text-lg font-semibold tracking-tight text-text-primary">
				<img src="/logo-dark.svg" alt="" class="h-6 w-6" />
				Cospan
			</a>
			<div class="hidden items-center gap-4 sm:flex">
				<a href="/" class="text-sm text-text-secondary hover:text-text-primary transition-colors">
					Explore
				</a>
				<a href="/feed" class="text-sm text-text-secondary hover:text-text-primary transition-colors">
					Feed
				</a>
				<a href="/orgs" class="text-sm text-text-secondary hover:text-text-primary transition-colors">
					Orgs
				</a>
				<a href="/search" class="text-sm text-text-secondary hover:text-text-primary transition-colors">
					Search
				</a>
			</div>
		</div>
		<div class="flex items-center gap-4">
			<div class="relative hidden sm:block">
				<svg
					class="pointer-events-none absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-text-secondary"
					fill="none"
					viewBox="0 0 24 24"
					stroke="currentColor"
					stroke-width="2"
				>
					<path
						stroke-linecap="round"
						stroke-linejoin="round"
						d="M21 21l-5.197-5.197m0 0A7.5 7.5 0 105.196 5.196a7.5 7.5 0 0010.607 10.607z"
					/>
				</svg>
				<input
					bind:this={searchInputRef}
					type="text"
					bind:value={searchValue}
					oninput={() => submitSearch(searchValue)}
					onkeydown={handleSearchKeydown}
					placeholder="Search... (/)"
					class="w-48 rounded-md border border-border bg-surface-0 py-1 pl-8 pr-3 text-xs text-text-primary placeholder:text-text-secondary focus:border-accent focus:outline-none"
				/>
			</div>
			<NotificationBell />
			{#if user?.authenticated}
				<a
					href="/new"
					class="rounded-md border border-border bg-surface-1 p-1.5 text-text-secondary transition-colors hover:border-accent hover:text-text-primary"
					title="New repository"
				>
					<svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
						<path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
					</svg>
				</a>
			{/if}
			<UserMenu {user} />
		</div>
	</nav>
</header>
