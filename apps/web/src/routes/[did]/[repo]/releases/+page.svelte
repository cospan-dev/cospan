<script lang="ts">
	import { getContext } from 'svelte';
	import BackLink from '$lib/components/shared/BackLink.svelte';
	import EmptyState from '$lib/components/shared/EmptyState.svelte';
	import { timeAgo } from '$lib/utils/time.js';
	import { getAuth } from '$lib/stores/auth.svelte';

	let { data } = $props();

	let auth = $derived(getAuth());
	let basePath = $derived(`/${data.did}/${data.repo}`);

	const repoLayout = getContext<any>('repoLayout');
	$effect(() => {
		repoLayout?.setExtraCrumbs([{ label: 'Releases' }]);
		return () => repoLayout?.setExtraCrumbs([]);
	});
</script>

<svelte:head>
	<title>Releases · {data.repo} · Cospan</title>
</svelte:head>

<div class="mt-3 mb-6 flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
	<h1 class="text-xl font-semibold text-text-primary">Releases</h1>
	{#if auth.authenticated}
		<a
			href="{basePath}/releases/new"
			class="rounded-md bg-accent px-3 py-1.5 text-center text-sm font-medium text-white transition-colors hover:bg-accent-hover"
		>
			New release
		</a>
	{/if}
</div>

{#if data.releases.items.length === 0}
	<EmptyState
		icon="release"
		message="No releases yet."
		ctaHref={auth.authenticated ? `${basePath}/releases/new` : undefined}
		ctaLabel={auth.authenticated ? 'Create the first release' : undefined}
	/>
{:else}
	<div class="space-y-4">
		{#each data.releases.items as release (release.rkey)}
			<a
				href="{basePath}/releases/{release.tag}"
				class="block rounded-lg border border-border bg-surface-1 p-4 transition-colors hover:border-accent"
			>
				<div class="flex items-start justify-between gap-3">
					<div class="min-w-0 flex-1">
						<div class="flex flex-wrap items-center gap-2">
							<h3 class="font-medium text-text-primary">{release.title}</h3>
							{#if release.prerelease}
								<span class="rounded-full bg-conflict/15 px-1.5 py-0.5 text-xs text-conflict">pre-release</span>
							{/if}
							{#if release.draft}
								<span class="rounded-full bg-surface-2 px-1.5 py-0.5 text-xs text-text-secondary">draft</span>
							{/if}
						</div>
						<div class="mt-1.5 flex flex-wrap items-center gap-3 text-xs text-text-secondary">
							<span class="rounded bg-surface-2 px-1.5 py-0.5 font-mono text-accent">{release.tag}</span>
							<span>{timeAgo(release.createdAt)}</span>
							{#if release.creatorHandle}
								<span>by {release.creatorHandle}</span>
							{/if}
						</div>
						{#if release.body}
							<p class="mt-2 text-sm text-text-secondary line-clamp-2">{release.body}</p>
						{/if}
					</div>
					{#if release.artifacts?.length > 0}
						<div class="shrink-0 text-right text-xs text-text-secondary">
							<span>{release.artifacts.length} artifact{release.artifacts.length !== 1 ? 's' : ''}</span>
						</div>
					{/if}
				</div>
			</a>
		{/each}
	</div>
{/if}

<BackLink href={basePath} />
