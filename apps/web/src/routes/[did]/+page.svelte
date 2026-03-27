<script lang="ts">
	import RepoCard from '$lib/components/repo/RepoCard.svelte';
	import FollowButton from '$lib/components/shared/FollowButton.svelte';
	import { page } from '$app/stores';

	let { data } = $props();

	let currentPath = $derived($page.url.pathname);
	let basePath = $derived(`/${data.did}`);

	/** Determine which tab is active based on the current URL path. */
	let activeTab = $derived.by(() => {
		if (currentPath.endsWith('/stars')) return 'stars';
		if (currentPath.endsWith('/followers')) return 'followers';
		if (currentPath.endsWith('/following')) return 'following';
		return 'repos';
	});
</script>

<svelte:head>
	<title>{data.profile?.displayName ?? data.profile?.handle ?? data.did} - Cospan</title>
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
		<div class="flex-1">
			<div class="flex items-center gap-3">
				<h1 class="text-2xl font-semibold text-text-primary">
					{data.profile?.displayName ?? data.profile?.handle ?? data.did}
				</h1>
				<FollowButton subjectDid={data.did} followerCount={data.profile?.followerCount ?? 0} />
			</div>
			{#if data.profile?.displayName && data.profile?.handle}
				<p class="text-sm text-text-secondary">@{data.profile.handle}</p>
			{/if}
			{#if data.profile?.description}
				<p class="mt-2 text-sm text-text-secondary">{data.profile.description}</p>
			{/if}
			<div class="mt-2 flex items-center gap-4 text-xs text-text-secondary">
				<a href="{basePath}/followers" class="hover:text-text-primary transition-colors">
					<span class="font-medium text-text-primary">{data.profile?.followerCount ?? 0}</span> followers
				</a>
				<a href="{basePath}/following" class="hover:text-text-primary transition-colors">
					<span class="font-medium text-text-primary">{data.profile?.followingCount ?? 0}</span> following
				</a>
				<span>
					<span class="font-medium text-text-primary">{data.profile?.repoCount ?? data.repos.items.length}</span> repos
				</span>
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

	<!-- Repositories tab content (default) -->
	{#if data.repos.items.length === 0}
		<p class="py-8 text-center text-sm text-text-secondary">No repositories yet.</p>
	{:else}
		<div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
			{#each data.repos.items as repo (repo.did + '/' + repo.name)}
				<RepoCard {repo} />
			{/each}
		</div>
	{/if}
</section>
