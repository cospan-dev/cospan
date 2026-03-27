<script lang="ts">
	import { page } from '$app/stores';
	import { timeAgo } from '$lib/utils/time.js';

	let { data } = $props();

	let basePath = $derived(`/${data.did}`);
	let currentPath = $derived($page.url.pathname);

	let activeTab = $derived.by(() => {
		if (currentPath.endsWith('/stars')) return 'stars';
		if (currentPath.endsWith('/followers')) return 'followers';
		if (currentPath.endsWith('/following')) return 'following';
		return 'repos';
	});
</script>

<svelte:head>
	<title>Stars - {data.profile?.displayName ?? data.profile?.handle ?? data.did} - Cospan</title>
</svelte:head>

<section>
	<div class="mb-8 flex items-start gap-5">
		{#if data.profile?.avatar}
			<img src={data.profile.avatar} alt="" class="h-16 w-16 rounded-full" />
		{:else}
			<div class="flex h-16 w-16 items-center justify-center rounded-full bg-surface-2 text-xl font-medium text-text-secondary">
				{(data.profile?.handle ?? data.did).charAt(0).toUpperCase()}
			</div>
		{/if}
		<div>
			<h1 class="text-2xl font-semibold text-text-primary">
				{data.profile?.displayName ?? data.profile?.handle ?? data.did}
			</h1>
			{#if data.profile?.displayName && data.profile?.handle}
				<p class="text-sm text-text-secondary">@{data.profile.handle}</p>
			{/if}
			<div class="mt-2 flex items-center gap-4 text-xs text-text-secondary">
				<a href="{basePath}/followers" class="hover:text-text-primary transition-colors">
					<span class="font-medium text-text-primary">{data.profile?.followerCount ?? 0}</span> followers
				</a>
				<a href="{basePath}/following" class="hover:text-text-primary transition-colors">
					<span class="font-medium text-text-primary">{data.profile?.followingCount ?? 0}</span> following
				</a>
			</div>
		</div>
	</div>

	<!-- Tab navigation -->
	<div class="mb-6 flex items-center gap-1 border-b border-border">
		<a
			href={basePath}
			class="border-b-2 px-4 py-2 text-sm font-medium transition-colors
				{activeTab === 'repos'
					? 'border-accent text-text-primary'
					: 'border-transparent text-text-secondary hover:text-text-primary'}"
		>
			Repositories
		</a>
		<a
			href="{basePath}/stars"
			class="border-b-2 px-4 py-2 text-sm font-medium transition-colors
				{activeTab === 'stars'
					? 'border-accent text-text-primary'
					: 'border-transparent text-text-secondary hover:text-text-primary'}"
		>
			Stars
			{#if data.stars.totalCount > 0}
				<span class="ml-1 rounded-full bg-surface-2 px-1.5 py-0.5 text-xs">{data.stars.totalCount}</span>
			{/if}
		</a>
		<a
			href="{basePath}/followers"
			class="border-b-2 px-4 py-2 text-sm font-medium transition-colors
				{activeTab === 'followers'
					? 'border-accent text-text-primary'
					: 'border-transparent text-text-secondary hover:text-text-primary'}"
		>
			Followers
		</a>
		<a
			href="{basePath}/following"
			class="border-b-2 px-4 py-2 text-sm font-medium transition-colors
				{activeTab === 'following'
					? 'border-accent text-text-primary'
					: 'border-transparent text-text-secondary hover:text-text-primary'}"
		>
			Following
		</a>
	</div>

	<!-- Starred repos -->
	{#if data.stars.items.length === 0}
		<div class="flex flex-col items-center gap-3 py-12 text-text-secondary">
			<svg class="h-10 w-10" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
				<path stroke-linecap="round" stroke-linejoin="round" d="M11.48 3.499a.562.562 0 011.04 0l2.125 5.111a.563.563 0 00.475.345l5.518.442c.499.04.701.663.321.988l-4.204 3.602a.563.563 0 00-.182.557l1.285 5.385a.562.562 0 01-.84.61l-4.725-2.885a.563.563 0 00-.586 0L6.982 20.54a.562.562 0 01-.84-.61l1.285-5.386a.562.562 0 00-.182-.557l-4.204-3.602a.563.563 0 01.321-.988l5.518-.442a.563.563 0 00.475-.345L11.48 3.5z" />
			</svg>
			<p>No starred repositories yet.</p>
		</div>
	{:else}
		<div class="space-y-3">
			{#each data.stars.items as star (star.rkey)}
				<a
					href="/{star.subject.replace('at://', '').replace('/dev.cospan.repo/', '/')}"
					class="block rounded-lg border border-border bg-surface-1 p-4 transition-colors hover:border-accent"
				>
					<div class="flex items-center justify-between">
						<div class="min-w-0 flex-1">
							<h3 class="font-mono text-sm font-medium text-text-primary truncate">
								{star.subject.replace('at://', '').replace('/dev.cospan.repo/', '/')}
							</h3>
							<p class="mt-1 text-xs text-text-secondary">
								Starred {timeAgo(star.createdAt)}
							</p>
						</div>
						<svg class="h-4 w-4 shrink-0 text-conflict" viewBox="0 0 24 24" fill="currentColor">
							<path d="M11.48 3.499a.562.562 0 011.04 0l2.125 5.111a.563.563 0 00.475.345l5.518.442c.499.04.701.663.321.988l-4.204 3.602a.563.563 0 00-.182.557l1.285 5.385a.562.562 0 01-.84.61l-4.725-2.885a.563.563 0 00-.586 0L6.982 20.54a.562.562 0 01-.84-.61l1.285-5.386a.562.562 0 00-.182-.557l-4.204-3.602a.563.563 0 01.321-.988l5.518-.442a.563.563 0 00.475-.345L11.48 3.5z" />
						</svg>
					</div>
				</a>
			{/each}
		</div>
	{/if}
</section>
