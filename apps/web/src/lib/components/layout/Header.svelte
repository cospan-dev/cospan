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

<header class="sticky top-0 z-50 border-b border-line/60 bg-void/80 backdrop-blur-xl">
	<nav class="mx-auto flex h-14 max-w-[1200px] items-center justify-between px-6">
		<!-- Wordmark -->
		<a href="/" class="group flex items-center gap-2 font-mono text-[15px] font-medium tracking-tight text-ink">
			<!-- Cospan mark: three vertices, the apex brighter -->
			<svg class="h-[18px] w-[18px]" viewBox="0 0 18 18" fill="none">
				<circle cx="3" cy="14" r="1.8" fill="currentColor" opacity="0.3"/>
				<circle cx="9" cy="3" r="2" fill="currentColor" opacity="0.7"/>
				<circle cx="15" cy="14" r="1.8" fill="currentColor" opacity="0.3"/>
				<line x1="3" y1="14" x2="9" y2="3" stroke="currentColor" stroke-width="0.8" opacity="0.2"/>
				<line x1="15" y1="14" x2="9" y2="3" stroke="currentColor" stroke-width="0.8" opacity="0.2"/>
			</svg>
			cospan
		</a>

		<!-- Nav -->
		<div class="flex items-center gap-1">
			{#each navLinks as link}
				<a
					href={link.href}
					class="relative rounded-md px-3.5 py-1.5 text-[13px] font-medium transition-all duration-150
						{isActive(link.href)
							? 'bg-raised text-ink'
							: 'text-ghost hover:text-caption hover:bg-surface'}"
				>
					{link.label}
				</a>
			{/each}
		</div>

		<!-- Right -->
		<div class="flex items-center gap-2">
			{#if user?.authenticated}
				<a
					href="/new"
					class="flex items-center gap-1.5 rounded-md border border-line px-3 py-1.5 text-[12px] font-medium text-caption transition-all hover:border-line-bright hover:text-ink"
				>
					<svg class="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2.5">
						<path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
					</svg>
					New
				</a>
			{/if}
			<UserMenu {user} />
		</div>
	</nav>
</header>
