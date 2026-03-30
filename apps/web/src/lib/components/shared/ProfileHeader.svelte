<script lang="ts">
	import FollowButton from './FollowButton.svelte';
	import { page } from '$app/stores';

	let {
		profile,
		did,
		showFollow = false
	}: {
		profile: {
			displayName?: string | null;
			handle?: string | null;
			avatarUrl?: string | null;
			description?: string | null;
			followerCount?: number;
			followingCount?: number;
			repoCount?: number;
		} | null;
		did: string;
		showFollow?: boolean;
	} = $props();

	let basePath = $derived(`/${did}`);
	let currentPath = $derived($page.url.pathname);

	let activeTab = $derived.by(() => {
		if (currentPath.endsWith('/stars')) return 'stars';
		if (currentPath.endsWith('/followers')) return 'followers';
		if (currentPath.endsWith('/following')) return 'following';
		return 'repos';
	});
</script>

<div class="mb-8">
	<!-- Avatar + identity -->
	<div class="flex flex-col gap-4 sm:flex-row sm:items-start sm:gap-5">
		{#if profile?.avatarUrl}
			<img src={profile.avatarUrl} alt="" class="h-16 w-16 rounded-full ring-2 ring-border" />
		{:else}
			<div class="flex h-16 w-16 shrink-0 items-center justify-center rounded-full bg-surface-2 text-xl font-medium text-text-secondary ring-2 ring-border">
				{(profile?.handle ?? did).charAt(0).toUpperCase()}
			</div>
		{/if}
		<div class="min-w-0 flex-1">
			<div class="flex items-center gap-3">
				<h1 class="text-2xl font-semibold text-text-primary">
					{profile?.displayName ?? profile?.handle ?? did}
				</h1>
				{#if showFollow}
					<FollowButton subjectDid={did} followerCount={profile?.followerCount ?? 0} />
				{/if}
			</div>
			{#if profile?.displayName && profile?.handle}
				<p class="mt-0.5 text-sm text-text-muted">@{profile.handle}</p>
			{/if}
			{#if profile?.description}
				<p class="mt-2 text-sm text-text-secondary leading-relaxed">{profile.description}</p>
			{/if}

			<!-- Stats row -->
			<div class="mt-3 flex items-center gap-5 text-sm">
				{#if profile?.repoCount !== undefined}
					<span class="text-text-secondary">
						<span class="font-semibold text-text-primary">{profile.repoCount}</span> repos
					</span>
				{/if}
				<a href="{basePath}/followers" class="text-text-secondary transition-colors hover:text-text-primary">
					<span class="font-semibold text-text-primary">{profile?.followerCount ?? 0}</span> followers
				</a>
				<a href="{basePath}/following" class="text-text-secondary transition-colors hover:text-text-primary">
					<span class="font-semibold text-text-primary">{profile?.followingCount ?? 0}</span> following
				</a>
			</div>
		</div>
	</div>

	<!-- Tab navigation -->
	<div class="mt-6 flex items-center gap-1 overflow-x-auto border-b border-border scrollbar-none">
		<a
			href={basePath}
			class="whitespace-nowrap border-b-2 px-4 py-2.5 text-sm font-medium transition-colors
				{activeTab === 'repos'
					? 'border-accent text-text-primary'
					: 'border-transparent text-text-muted hover:text-text-primary'}"
		>
			Repositories
		</a>
		<a
			href="{basePath}/stars"
			class="whitespace-nowrap border-b-2 px-4 py-2.5 text-sm font-medium transition-colors
				{activeTab === 'stars'
					? 'border-accent text-text-primary'
					: 'border-transparent text-text-muted hover:text-text-primary'}"
		>
			Stars
		</a>
		<a
			href="{basePath}/followers"
			class="whitespace-nowrap border-b-2 px-4 py-2.5 text-sm font-medium transition-colors
				{activeTab === 'followers'
					? 'border-accent text-text-primary'
					: 'border-transparent text-text-muted hover:text-text-primary'}"
		>
			Followers
		</a>
		<a
			href="{basePath}/following"
			class="whitespace-nowrap border-b-2 px-4 py-2.5 text-sm font-medium transition-colors
				{activeTab === 'following'
					? 'border-accent text-text-primary'
					: 'border-transparent text-text-muted hover:text-text-primary'}"
		>
			Following
		</a>
	</div>
</div>
