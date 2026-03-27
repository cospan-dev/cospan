<script lang="ts">
	import { timeAgo } from '$lib/utils/time.js';

	let { data } = $props();
</script>

<svelte:head>
	<title>Organizations - Cospan</title>
</svelte:head>

<section>
	<h1 class="mb-6 text-2xl font-semibold text-text-primary">Organizations</h1>

	{#if data.orgs.items.length === 0}
		<div class="flex flex-col items-center gap-4 py-12 text-text-secondary">
			<svg class="h-12 w-12" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
				<path stroke-linecap="round" stroke-linejoin="round" d="M18 18.72a9.094 9.094 0 003.741-.479 3 3 0 00-4.682-2.72m.94 3.198l.001.031c0 .225-.012.447-.037.666A11.944 11.944 0 0112 21c-2.17 0-4.207-.576-5.963-1.584A6.062 6.062 0 016 18.719m12 0a5.971 5.971 0 00-.941-3.197m0 0A5.995 5.995 0 0012 12.75a5.995 5.995 0 00-5.058 2.772m0 0a3 3 0 00-4.681 2.72 8.986 8.986 0 003.74.477m.94-3.197a5.971 5.971 0 00-.94 3.197M15 6.75a3 3 0 11-6 0 3 3 0 016 0zm6 3a2.25 2.25 0 11-4.5 0 2.25 2.25 0 014.5 0zm-13.5 0a2.25 2.25 0 11-4.5 0 2.25 2.25 0 014.5 0z" />
			</svg>
			<p>No organizations yet.</p>
		</div>
	{:else}
		<div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
			{#each data.orgs.items as org (org.did + '/' + org.rkey)}
				<a
					href="/orgs/{org.did}/{org.rkey}"
					class="block rounded-lg border border-border bg-surface-1 p-4 transition-colors hover:border-accent"
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
