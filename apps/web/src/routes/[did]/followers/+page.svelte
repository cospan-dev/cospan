<script lang="ts">
	import { page } from '$app/stores';

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
	<title>Followers - {data.profile?.displayName ?? data.profile?.handle ?? data.did} - Cospan</title>
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
		</a>
		<a
			href="{basePath}/followers"
			class="border-b-2 px-4 py-2 text-sm font-medium transition-colors
				{activeTab === 'followers'
					? 'border-accent text-text-primary'
					: 'border-transparent text-text-secondary hover:text-text-primary'}"
		>
			Followers
			{#if data.followers.totalCount > 0}
				<span class="ml-1 rounded-full bg-surface-2 px-1.5 py-0.5 text-xs">{data.followers.totalCount}</span>
			{/if}
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

	<!-- Followers list -->
	{#if data.followers.items.length === 0}
		<div class="flex flex-col items-center gap-3 py-12 text-text-secondary">
			<svg class="h-10 w-10" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
				<path stroke-linecap="round" stroke-linejoin="round" d="M15 19.128a9.38 9.38 0 002.625.372 9.337 9.337 0 004.121-.952 4.125 4.125 0 00-7.533-2.493M15 19.128v-.003c0-1.113-.285-2.16-.786-3.07M15 19.128v.106A12.318 12.318 0 018.624 21c-2.331 0-4.512-.645-6.374-1.766l-.001-.109a6.375 6.375 0 0111.964-3.07M12 6.375a3.375 3.375 0 11-6.75 0 3.375 3.375 0 016.75 0zm8.25 2.25a2.625 2.625 0 11-5.25 0 2.625 2.625 0 015.25 0z" />
			</svg>
			<p>No followers yet.</p>
		</div>
	{:else}
		<div class="space-y-2">
			{#each data.followers.items as user (user.did)}
				<a
					href="/{user.did}"
					class="flex items-center gap-3 rounded-lg border border-border bg-surface-1 px-4 py-3 transition-colors hover:border-accent"
				>
					{#if user.avatarUrl}
						<img src={user.avatarUrl} alt="" class="h-10 w-10 rounded-full" />
					{:else}
						<div class="flex h-10 w-10 items-center justify-center rounded-full bg-surface-2 text-sm font-medium text-text-secondary">
							{(user.handle ?? user.did).charAt(0).toUpperCase()}
						</div>
					{/if}
					<div class="min-w-0 flex-1">
						{#if user.displayName}
							<p class="text-sm font-medium text-text-primary">{user.displayName}</p>
						{/if}
						<p class="text-xs text-text-secondary truncate">
							{user.handle ? `@${user.handle}` : user.did}
						</p>
					</div>
				</a>
			{/each}
		</div>
	{/if}
</section>
