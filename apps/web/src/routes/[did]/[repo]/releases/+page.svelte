<script lang="ts">
	import Breadcrumb from '$lib/components/shared/Breadcrumb.svelte';
	import { timeAgo } from '$lib/utils/time.js';
	import { getAuth } from '$lib/stores/auth.svelte';

	let { data } = $props();

	let auth = $derived(getAuth());
	let basePath = $derived(`/${data.did}/${data.repo}`);

	let crumbs = $derived([
		{ label: data.did, href: `/${data.did}` },
		{ label: data.repo, href: basePath },
		{ label: 'Releases' },
	]);

	function formatSize(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
		return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
	}
</script>

<svelte:head>
	<title>Releases - {data.repo} - Cospan</title>
</svelte:head>

<section>
	<div class="mb-6">
		<Breadcrumb {crumbs} />

		<div class="mt-3 flex items-center justify-between">
			<h1 class="text-xl font-semibold text-text-primary">Releases</h1>
			{#if auth.authenticated}
				<a
					href="{basePath}/releases/new"
					class="rounded-md bg-accent px-3 py-1.5 text-sm font-medium text-white transition-colors hover:bg-accent-hover"
				>
					New release
				</a>
			{/if}
		</div>
	</div>

	{#if data.releases.items.length === 0}
		<div class="flex flex-col items-center gap-4 py-12 text-text-secondary">
			<svg class="h-12 w-12" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
				<path stroke-linecap="round" stroke-linejoin="round" d="M9.568 3H5.25A2.25 2.25 0 003 5.25v4.318c0 .597.237 1.17.659 1.591l9.581 9.581c.699.699 1.78.872 2.607.33a18.095 18.095 0 005.223-5.223c.542-.827.369-1.908-.33-2.607L11.16 3.66A2.25 2.25 0 009.568 3z" />
				<path stroke-linecap="round" stroke-linejoin="round" d="M6 6h.008v.008H6V6z" />
			</svg>
			<p>No releases yet.</p>
			{#if auth.authenticated}
				<a
					href="{basePath}/releases/new"
					class="rounded-md bg-accent px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-accent-hover"
				>
					Create the first release
				</a>
			{/if}
		</div>
	{:else}
		<div class="space-y-4">
			{#each data.releases.items as release (release.rkey)}
				<a
					href="{basePath}/releases/{release.tag}"
					class="block rounded-lg border border-border bg-surface-1 p-4 transition-colors hover:border-accent"
				>
					<div class="flex items-start justify-between gap-3">
						<div class="min-w-0 flex-1">
							<div class="flex items-center gap-2">
								<h3 class="font-medium text-text-primary">{release.title}</h3>
								{#if release.prerelease}
									<span class="rounded-full bg-conflict/15 px-1.5 py-0.5 text-xs text-conflict">pre-release</span>
								{/if}
								{#if release.draft}
									<span class="rounded-full bg-surface-2 px-1.5 py-0.5 text-xs text-text-secondary">draft</span>
								{/if}
							</div>
							<div class="mt-1.5 flex items-center gap-3 text-xs text-text-secondary">
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
						{#if release.artifacts.length > 0}
							<div class="shrink-0 text-right text-xs text-text-secondary">
								<span>{release.artifacts.length} artifact{release.artifacts.length !== 1 ? 's' : ''}</span>
							</div>
						{/if}
					</div>
				</a>
			{/each}
		</div>
	{/if}
</section>
