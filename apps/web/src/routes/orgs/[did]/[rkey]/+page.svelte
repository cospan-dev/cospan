<script lang="ts">
	import { timeAgo } from '$lib/utils/time.js';

	let { data } = $props();
</script>

<svelte:head>
	<title>{data.org.name}· Organizations · Cospan</title>
</svelte:head>

<section>
	<div class="mb-8">
		<div class="flex items-center gap-2 text-sm text-text-muted">
			<a href="/orgs" class="text-accent hover:text-accent-hover transition-colors">
				Organizations
			</a>
			<span>/</span>
			<span class="text-text-secondary">{data.org.name}</span>
		</div>

		<div class="mt-4 flex flex-col items-start gap-4 sm:flex-row sm:items-center">
			{#if data.org.avatarUrl}
				<img
					src={data.org.avatarUrl}
					alt={data.org.name}
					class="h-16 w-16 rounded-full bg-surface-2"
				/>
			{:else}
				<div class="flex h-16 w-16 items-center justify-center rounded-full bg-surface-2 text-xl font-medium text-text-secondary">
					{data.org.name.charAt(0).toUpperCase()}
				</div>
			{/if}
			<div>
				<h1 class="text-2xl font-semibold text-text-primary">{data.org.name}</h1>
				{#if data.org.description}
					<p class="mt-1 text-sm text-text-secondary">{data.org.description}</p>
				{/if}
				<div class="mt-2 flex gap-4 text-xs text-text-secondary">
					<span>{data.org.memberCount} members</span>
					<span>{data.org.repoCount} repos</span>
					<span>created {timeAgo(data.org.createdAt)}</span>
				</div>
			</div>
		</div>
	</div>

	<div class="rounded-lg border border-border bg-surface-1">
		<div class="border-b border-border px-4 py-3">
			<h2 class="text-sm font-medium text-text-primary">
				Members ({data.members.items.length})
			</h2>
		</div>

		{#if data.members.items.length === 0}
			<p class="px-4 py-8 text-center text-sm text-text-secondary">No members.</p>
		{:else}
			<ul class="divide-y divide-border">
				{#each data.members.items as member (member.did)}
					<li class="flex flex-col gap-2 px-4 py-3 sm:flex-row sm:items-center sm:justify-between">
						<div class="flex items-center gap-3">
							<a
								href="/{member.did}"
								class="text-sm font-medium text-text-primary hover:text-accent transition-colors"
							>
								{member.displayName ?? member.handle ?? member.did}
							</a>
							{#if member.displayName && member.handle}
								<span class="text-xs text-text-secondary">{member.handle}</span>
							{/if}
						</div>
						<div class="flex items-center gap-3">
							<span class="rounded-full bg-surface-2 px-2 py-0.5 text-xs text-text-secondary">
								{member.role}
							</span>
							<span class="text-xs text-text-secondary">
								joined {timeAgo(member.joinedAt)}
							</span>
						</div>
					</li>
				{/each}
			</ul>
		{/if}
	</div>
</section>
