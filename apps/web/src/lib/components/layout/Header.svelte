<script lang="ts">
	import { page } from '$app/stores';
	import UserMenu from '$lib/components/auth/UserMenu.svelte';

	let { user }: { user?: { authenticated: boolean; did: string; handle: string; displayName?: string; avatar?: string } | null } = $props();

	let currentPath = $derived($page.url.pathname);

	const navLinks = [
		{ href: '/', label: 'Explore' },
		{ href: '/feed', label: 'Feed' },
		{ href: '/search', label: 'Search' },
	] as const;

	function isActive(href: string): boolean {
		if (href === '/') return currentPath === '/';
		return currentPath.startsWith(href);
	}
</script>

<header class="graph-paper border-b border-border bg-bg">
	<nav class="mx-auto flex h-12 max-w-6xl items-center justify-between px-4">
		<!-- Left: wordmark -->
		<a href="/" class="flex items-center gap-1.5 text-base font-medium tracking-tight text-text-primary">
			<!-- Cospan arc: a subtle SVG arc above the 'o' evoking the cospan diagram apex -->
			<svg class="h-5 w-5" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
				<path d="M4 18 C4 18 8 4 12 4 C16 4 20 18 20 18" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" fill="none" opacity="0.6" />
				<circle cx="4" cy="18" r="2" fill="currentColor" opacity="0.4" />
				<circle cx="12" cy="4" r="2" fill="currentColor" opacity="0.7" />
				<circle cx="20" cy="18" r="2" fill="currentColor" opacity="0.4" />
			</svg>
			<span>cospan</span>
		</a>

		<!-- Center: nav links -->
		<div class="flex items-center gap-6">
			{#each navLinks as link}
				<a
					href={link.href}
					class="relative text-sm transition-colors duration-150
						{isActive(link.href)
							? 'text-accent'
							: 'text-text-muted hover:text-text-secondary'}"
				>
					{link.label}
					{#if isActive(link.href)}
						<span class="absolute -bottom-[13px] left-0 right-0 h-[2px] bg-accent"></span>
					{/if}
				</a>
			{/each}
		</div>

		<!-- Right: avatar or sign in -->
		<div class="flex items-center gap-3">
			{#if user?.authenticated}
				<a
					href="/new"
					class="rounded-md border border-border p-1.5 text-text-muted transition-colors hover:border-border-hover hover:text-text-secondary"
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
