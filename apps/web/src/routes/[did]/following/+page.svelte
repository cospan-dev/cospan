<script lang="ts">
	import ProfileHeader from '$lib/components/shared/ProfileHeader.svelte';
	import EmptyState from '$lib/components/shared/EmptyState.svelte';

	let { data } = $props();
</script>

<svelte:head>
	<title>Following · {data.profile?.displayName ?? data.profile?.handle ?? data.did} · Cospan</title>
</svelte:head>

<section>
	<ProfileHeader profile={data.profile} did={data.did} />

	<!-- Following list -->
	{#if data.following.items.length === 0}
		<EmptyState icon="users" message="Not following anyone yet." />
	{:else}
		<div class="space-y-2">
			{#each data.following.items as user (user.did)}
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
