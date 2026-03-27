<script lang="ts">
	import Breadcrumb from '$lib/components/shared/Breadcrumb.svelte';
	import { timeAgo, formatDate } from '$lib/utils/time.js';

	let { data } = $props();

	let basePath = $derived(`/${data.did}/${data.repoName}`);

	let crumbs = $derived([
		{ label: data.did, href: `/${data.did}` },
		{ label: data.repoName, href: basePath },
		{ label: `Commit ${data.commitId.slice(0, 10)}` }
	]);

	function truncateHash(hash: string): string {
		return hash.slice(0, 10);
	}
</script>

<svelte:head>
	<title>Commit {data.commitId.slice(0, 10)} - {data.repoName} - Cospan</title>
</svelte:head>

<section>
	<Breadcrumb {crumbs} />

	{#if data.commit}
		<div class="mt-4">
			<!-- Commit header -->
			<div class="rounded-lg border border-border bg-surface-1 p-5">
				{#if data.commit.message}
					<h1 class="text-lg font-semibold text-text-primary">{data.commit.message}</h1>
				{:else}
					<h1 class="text-lg font-semibold text-text-secondary italic">No commit message</h1>
				{/if}

				<div class="mt-3 flex flex-wrap items-center gap-x-4 gap-y-1 text-sm text-text-secondary">
					{#if data.commit.author}
						<span>
							Author: <span class="font-mono text-text-primary">{data.commit.author}</span>
						</span>
					{/if}
					{#if data.commit.committer && data.commit.committer !== data.commit.author}
						<span>
							Committer: <span class="font-mono text-text-primary">{data.commit.committer}</span>
						</span>
					{/if}
					{#if data.commit.timestamp}
						<time datetime={data.commit.timestamp} title={formatDate(data.commit.timestamp)}>
							{timeAgo(data.commit.timestamp)}
						</time>
					{/if}
				</div>

				<div class="mt-3 flex items-center gap-2">
					<span class="text-xs text-text-secondary">Commit</span>
					<code class="rounded bg-surface-2 px-2 py-0.5 font-mono text-xs text-text-primary">
						{data.commit.id}
					</code>
				</div>
			</div>

			<!-- Parents -->
			{#if data.commit.parents.length > 0}
				<div class="mt-4 rounded-lg border border-border bg-surface-1 p-4">
					<h2 class="mb-2 text-sm font-medium text-text-primary">
						{data.commit.parents.length === 1 ? 'Parent' : 'Parents'}
					</h2>
					<ul class="space-y-1">
						{#each data.commit.parents as parent (parent)}
							<li>
								<a
									href="{basePath}/commit/{parent}"
									class="font-mono text-xs text-accent transition-colors hover:text-accent-hover"
								>
									{truncateHash(parent)}
								</a>
							</li>
						{/each}
					</ul>
				</div>
			{/if}

			<!-- Object references -->
			<div class="mt-4 rounded-lg border border-border bg-surface-1 p-4">
				<h2 class="mb-3 text-sm font-medium text-text-primary">Objects</h2>
				<div class="space-y-2">
					{#if data.commit.schemaId}
						<div class="flex items-center gap-3">
							<span class="w-20 text-xs text-text-secondary">Schema</span>
							<code class="rounded bg-surface-2 px-2 py-0.5 font-mono text-xs text-text-primary">
								{data.commit.schemaId}
							</code>
						</div>
					{/if}
					{#if data.commit.migrationId}
						<div class="flex items-center gap-3">
							<span class="w-20 text-xs text-text-secondary">Migration</span>
							<code class="rounded bg-surface-2 px-2 py-0.5 font-mono text-xs text-text-primary">
								{data.commit.migrationId}
							</code>
						</div>
					{/if}
					<div class="flex items-center gap-3">
						<span class="w-20 text-xs text-text-secondary">Kind</span>
						<span class="rounded bg-surface-2 px-2 py-0.5 font-mono text-xs text-text-primary">
							{data.commit.kind}
						</span>
					</div>
				</div>
			</div>

			<!-- Raw data (collapsible) -->
			<details class="mt-4 rounded-lg border border-border bg-surface-1">
				<summary class="cursor-pointer px-4 py-3 text-sm font-medium text-text-primary hover:text-accent transition-colors">
					Raw object data
				</summary>
				<pre class="overflow-x-auto border-t border-border px-4 py-3 font-mono text-xs text-text-secondary">{data.commit.raw}</pre>
			</details>
		</div>
	{:else}
		<div class="mt-8 flex flex-col items-center gap-3 py-12 text-text-secondary">
			<p class="text-sm">Commit not found.</p>
			<p class="text-xs">
				Object <code class="font-mono">{data.commitId}</code> could not be loaded from the node.
			</p>
			<a
				href={basePath}
				class="mt-2 rounded-md border border-border bg-surface-1 px-4 py-2 text-xs text-text-secondary transition-colors hover:border-accent hover:text-text-primary"
			>
				Back to repository
			</a>
		</div>
	{/if}
</section>
