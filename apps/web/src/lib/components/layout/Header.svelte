<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import { debounce } from '$lib/utils/debounce.js';
	import UserMenu from '$lib/components/auth/UserMenu.svelte';
	import NotificationBell from '$lib/components/shared/NotificationBell.svelte';

	let { user }: { user?: { authenticated: boolean; did: string; handle: string; displayName?: string; avatar?: string } | null } = $props();

	let searchValue = $state('');
	let mobileSearchValue = $state('');
	let searchInputRef = $state<HTMLInputElement | null>(null);
	let mobileMenuOpen = $state(false);

	// Close mobile menu on navigation
	$effect(() => {
		$page.url.pathname;
		mobileMenuOpen = false;
	});

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

	function handleMobileSearchKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter' && mobileSearchValue.trim()) {
			goto(`/search?q=${encodeURIComponent(mobileSearchValue.trim())}`);
			mobileMenuOpen = false;
		}
	}

	function handleGlobalKeydown(event: KeyboardEvent) {
		const target = event.target as HTMLElement;
		if (
			target.tagName === 'INPUT' ||
			target.tagName === 'TEXTAREA' ||
			target.isContentEditable
		) {
			return;
		}

		if (event.key === '/') {
			event.preventDefault();
			searchInputRef?.focus();
		}

		if (event.key === 'Escape' && mobileMenuOpen) {
			mobileMenuOpen = false;
		}
	}

	function toggleMobileMenu() {
		mobileMenuOpen = !mobileMenuOpen;
	}
</script>

<svelte:window onkeydown={handleGlobalKeydown} />

<header class="border-b border-border bg-surface-1">
	<nav class="mx-auto flex h-12 max-w-6xl items-center justify-between px-4">
		<div class="flex items-center gap-6">
			<a href="/" class="flex items-center gap-2 text-lg font-semibold tracking-tight text-text-primary">
				<img src="/logo-dark.svg" alt="" class="h-6 w-6" />
				<span class="hidden xs:inline">Cospan</span>
			</a>
			<div class="hidden items-center gap-4 md:flex">
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
		<div class="flex items-center gap-3">
			<!-- Desktop search -->
			<div class="relative hidden md:block">
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
					class="hidden rounded-md border border-border bg-surface-1 p-1.5 text-text-secondary transition-colors hover:border-accent hover:text-text-primary sm:block"
					title="New repository"
				>
					<svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
						<path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
					</svg>
				</a>
			{/if}
			<div class="hidden sm:block">
				<UserMenu {user} />
			</div>

			<!-- Mobile hamburger -->
			<button
				onclick={toggleMobileMenu}
				class="rounded-md p-1.5 text-text-secondary transition-colors hover:bg-surface-2 hover:text-text-primary md:hidden"
				aria-label="Toggle menu"
			>
				{#if mobileMenuOpen}
					<svg class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
						<path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
					</svg>
				{:else}
					<svg class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
						<path stroke-linecap="round" stroke-linejoin="round" d="M3.75 6.75h16.5M3.75 12h16.5m-16.5 5.25h16.5" />
					</svg>
				{/if}
			</button>
		</div>
	</nav>

	<!-- Mobile menu -->
	{#if mobileMenuOpen}
		<div class="border-t border-border bg-surface-1 px-4 py-3 md:hidden">
			<!-- Mobile search -->
			<div class="mb-3">
				<div class="relative">
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
						type="text"
						bind:value={mobileSearchValue}
						onkeydown={handleMobileSearchKeydown}
						placeholder="Search repositories..."
						class="w-full rounded-md border border-border bg-surface-0 py-2 pl-8 pr-3 text-sm text-text-primary placeholder:text-text-secondary focus:border-accent focus:outline-none"
					/>
				</div>
			</div>

			<!-- Mobile nav links -->
			<div class="space-y-1">
				<a href="/" class="block rounded-md px-3 py-2 text-sm text-text-secondary transition-colors hover:bg-surface-2 hover:text-text-primary">
					Explore
				</a>
				<a href="/feed" class="block rounded-md px-3 py-2 text-sm text-text-secondary transition-colors hover:bg-surface-2 hover:text-text-primary">
					Feed
				</a>
				<a href="/orgs" class="block rounded-md px-3 py-2 text-sm text-text-secondary transition-colors hover:bg-surface-2 hover:text-text-primary">
					Orgs
				</a>
				<a href="/search" class="block rounded-md px-3 py-2 text-sm text-text-secondary transition-colors hover:bg-surface-2 hover:text-text-primary">
					Search
				</a>
			</div>

			{#if user?.authenticated}
				<div class="mt-3 border-t border-border pt-3 space-y-1">
					<a
						href="/new"
						class="flex items-center gap-2 rounded-md px-3 py-2 text-sm text-text-secondary transition-colors hover:bg-surface-2 hover:text-text-primary"
					>
						<svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
							<path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
						</svg>
						New repository
					</a>
					<a
						href="/{user.did}"
						class="block rounded-md px-3 py-2 text-sm text-text-secondary transition-colors hover:bg-surface-2 hover:text-text-primary"
					>
						Your profile
					</a>
					<a
						href="/settings"
						class="block rounded-md px-3 py-2 text-sm text-text-secondary transition-colors hover:bg-surface-2 hover:text-text-primary"
					>
						Settings
					</a>
				</div>
			{:else}
				<div class="mt-3 border-t border-border pt-3">
					<div class="sm:hidden">
						<UserMenu {user} />
					</div>
				</div>
			{/if}
		</div>
	{/if}
</header>
