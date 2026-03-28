<script lang="ts">
	import Breadcrumb from '$lib/components/shared/Breadcrumb.svelte';
	import BackLink from '$lib/components/shared/BackLink.svelte';
	import { formatDate } from '$lib/utils/time.js';

	let { data } = $props();

	let basePath = $derived(`/${data.did}/${data.repo}`);

	let crumbs = $derived([
		{ label: data.did, href: `/${data.did}` },
		{ label: data.repo, href: basePath },
		{ label: 'Releases', href: `${basePath}/releases` },
		{ label: data.release?.tag ?? 'Not found' },
	]);

	function formatSize(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
		return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
	}
</script>

<svelte:head>
	<title>{data.release?.title ?? 'Release'} - {data.repo} - Cospan</title>
</svelte:head>

<section>
	<Breadcrumb {crumbs} />

	{#if data.release}
		<div class="mt-6">
			<div class="flex items-center gap-3">
				<h1 class="text-2xl font-semibold text-text-primary">{data.release.title}</h1>
				{#if data.release.prerelease}
					<span class="rounded-full bg-conflict/15 px-2 py-0.5 text-xs font-medium text-conflict">pre-release</span>
				{/if}
				{#if data.release.draft}
					<span class="rounded-full bg-surface-2 px-2 py-0.5 text-xs font-medium text-text-secondary">draft</span>
				{/if}
			</div>

			<div class="mt-2 flex items-center gap-3 text-sm text-text-secondary">
				<span class="rounded bg-surface-2 px-1.5 py-0.5 font-mono text-xs text-accent">{data.release.tag}</span>
				<span>Published {formatDate(data.release.createdAt)}</span>
				{#if data.release.creatorHandle}
					<span>by <a href="/{data.release.creatorDid}" class="text-accent hover:text-accent-hover">{data.release.creatorHandle}</a></span>
				{/if}
			</div>
		</div>

		{#if data.release.body}
			<div class="mt-6 rounded-lg border border-border bg-surface-1 p-6">
				<div class="text-sm text-text-primary whitespace-pre-wrap">{data.release.body}</div>
			</div>
		{/if}

		{#if data.release.artifacts.length > 0}
			<div class="mt-6">
				<h2 class="mb-3 text-sm font-medium text-text-primary">Assets</h2>
				<div class="divide-y divide-border rounded-lg border border-border bg-surface-1">
					{#each data.release.artifacts as artifact (artifact.name)}
						<a
							href={artifact.downloadUrl}
							class="flex items-center justify-between px-4 py-3 transition-colors hover:bg-surface-2"
							download
						>
							<div class="flex items-center gap-2">
								<svg class="h-4 w-4 text-text-secondary" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
									<path stroke-linecap="round" stroke-linejoin="round" d="M3 16.5v2.25A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75V16.5M16.5 12L12 16.5m0 0L7.5 12m4.5 4.5V3" />
								</svg>
								<span class="font-mono text-sm text-text-primary">{artifact.name}</span>
							</div>
							<span class="text-xs text-text-secondary">{formatSize(artifact.size)}</span>
						</a>
					{/each}
				</div>
			</div>
		{:else}
			<div class="mt-6 rounded-lg border border-border bg-surface-1 px-4 py-8 text-center text-sm text-text-secondary">
				No assets attached to this release.
			</div>
		{/if}
	{:else}
		<div class="mt-6 rounded-lg border border-border bg-surface-1 p-8 text-center">
			<p class="text-text-secondary">Release not found.</p>
			<a
				href="{basePath}/releases"
				class="mt-3 inline-block text-sm text-accent transition-colors hover:text-accent-hover"
			>
				View all releases
			</a>
		</div>
	{/if}

	<BackLink href="{basePath}/releases" label="Back to releases" />
</section>
