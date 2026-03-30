<script lang="ts">
	import EmptyState from '$lib/components/shared/EmptyState.svelte';
	import { timeAgo } from '$lib/utils/time.js';

	let { data } = $props();
</script>

<svelte:head>
	<title>Organizations - Cospan</title>
</svelte:head>

<section>
	<h1 class="mb-1 text-xl font-semibold text-text-primary">Organizations</h1>
	<p class="mb-6 text-sm text-text-muted">Groups and teams on the network.</p>

	{#if data.orgs.items.length === 0}
		<EmptyState icon="users" message="No organizations yet." />
	{:else}
		<div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
			{#each data.orgs.items as org (org.did + '/' + org.rkey)}
				<a
					href="/orgs/{org.did}/{org.rkey}"
					class="block rounded-lg border border-border bg-surface-1 p-4 transition-all hover:border-border-hover"
				>
					<div class="flex items-center gap-3">
						{#if org.avatarUrl}
							<img
								src={org.avatarUrl}
								alt={org.name}
								class="h-10 w-10 rounded-full bg-surface-2"
							/>
						{:else}
							<div class="flex h-10 w-10 items-center justify-center rounded-full bg-surface-2 text-sm font-medium text-text-secondary">
								{org.name.charAt(0).toUpperCase()}
							</div>
						{/if}
						<div class="min-w-0 flex-1">
							<h3 class="font-medium text-text-primary">{org.name}</h3>
							{#if org.description}
								<p class="mt-0.5 truncate text-sm text-text-secondary">{org.description}</p>
							{/if}
						</div>
					</div>
					<div class="mt-3 flex items-center gap-4 text-xs text-text-secondary">
						<span>{org.memberCount} members</span>
						<span>{org.repoCount} repos</span>
						<span>created {timeAgo(org.createdAt)}</span>
					</div>
				</a>
			{/each}
		</div>
	{/if}
</section>
