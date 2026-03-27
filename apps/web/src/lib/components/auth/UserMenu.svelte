<script lang="ts">
	import LoginButton from './LoginButton.svelte';
	import { doLogout } from '$lib/stores/auth.svelte';

	let { user }: { user?: { authenticated: boolean; did: string; handle: string; displayName?: string; avatar?: string } | null } = $props();

	let menuOpen = $state(false);

	function toggleMenu() {
		menuOpen = !menuOpen;
	}

	function closeMenu() {
		menuOpen = false;
	}

	async function handleLogout() {
		await doLogout();
		window.location.reload();
	}

	function handleWindowClick(event: MouseEvent) {
		const target = event.target as HTMLElement;
		if (!target.closest('[data-user-menu]')) {
			menuOpen = false;
		}
	}
</script>

<svelte:window onclick={handleWindowClick} />

{#if user?.authenticated}
	<div class="relative" data-user-menu>
		<button
			onclick={toggleMenu}
			class="flex items-center gap-2 rounded-md px-2 py-1 text-sm text-text-primary transition-colors hover:bg-surface-2"
		>
			{#if user.avatar}
				<img src={user.avatar} alt="" class="h-7 w-7 rounded-full" />
			{:else}
				<div class="flex h-7 w-7 items-center justify-center rounded-full bg-accent text-xs font-medium text-white">
					{user.handle.charAt(0).toUpperCase()}
				</div>
			{/if}
			<svg class="h-3 w-3 text-text-secondary" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
				<path stroke-linecap="round" stroke-linejoin="round" d="M19 9l-7 7-7-7" />
			</svg>
		</button>

		{#if menuOpen}
			<div class="absolute right-0 top-full z-50 mt-1 w-56 rounded-lg border border-border bg-surface-1 py-1 shadow-lg">
				<div class="border-b border-border px-3 py-2">
					<p class="text-sm font-medium text-text-primary">{user.displayName ?? user.handle}</p>
					<p class="truncate text-xs text-text-secondary">@{user.handle}</p>
				</div>
				<a
					href="/{user.did}"
					onclick={closeMenu}
					class="block px-3 py-2 text-sm text-text-secondary transition-colors hover:bg-surface-2 hover:text-text-primary"
				>
					Your profile
				</a>
				<a
					href="/{user.did}"
					onclick={closeMenu}
					class="block px-3 py-2 text-sm text-text-secondary transition-colors hover:bg-surface-2 hover:text-text-primary"
				>
					Your repositories
				</a>
				<a
					href="/settings"
					onclick={closeMenu}
					class="block px-3 py-2 text-sm text-text-secondary transition-colors hover:bg-surface-2 hover:text-text-primary"
				>
					Settings
				</a>
				<div class="border-t border-border">
					<button
						onclick={handleLogout}
						class="block w-full px-3 py-2 text-left text-sm text-text-secondary transition-colors hover:bg-surface-2 hover:text-text-primary"
					>
						Sign out
					</button>
				</div>
			</div>
		{/if}
	</div>
{:else}
	<LoginButton />
{/if}
