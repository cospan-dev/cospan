<script lang="ts">
	import ProfileHeader from '$lib/components/shared/ProfileHeader.svelte';
	import EmptyState from '$lib/components/shared/EmptyState.svelte';
	import { timeAgo } from '$lib/utils/time.js';

	let { data } = $props();
</script>

<svelte:head>
	<title>Stars · {data.profile?.displayName ?? data.profile?.handle ?? data.did} · Cospan</title>
</svelte:head>

<section>
	<ProfileHeader profile={data.profile} did={data.did} />

	<!-- Starred repos -->
	{#if data.stars.items.length === 0}
		<EmptyState icon="star" message="No starred repositories yet." />
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
