<script lang="ts">
	import { getContext } from 'svelte';
	import BackLink from '$lib/components/shared/BackLink.svelte';
	import StructuralDiff from '$lib/components/repo/StructuralDiff.svelte';

	let { data } = $props();

	let basePath = $derived(`/${data.did}/${data.repoName}`);

	const repoLayout = getContext<any>('repoLayout');
	$effect(() => {
		repoLayout?.setExtraCrumbs([{ label: `Commit ${data.commitId.slice(0, 10)}` }]);
		return () => repoLayout?.setExtraCrumbs([]);
	});

	function truncateHash(hash: string): string {
		return hash.slice(0, 10);
	}

	function formatTime(unixSeconds: number): string {
		if (!unixSeconds) return '';
		const d = new Date(unixSeconds * 1000);
		return d.toLocaleString('en-US', {
			month: 'short',
			day: 'numeric',
			year: 'numeric',
			hour: '2-digit',
			minute: '2-digit',
		});
	}
</script>

<svelte:head>
	<title>Commit {data.commitId.slice(0, 10)} · {data.repoName} · Cospan</title>
</svelte:head>

{#if data.commit}
	<!-- Commit header -->
	<div class="rounded-lg border border-border bg-surface-1 p-5">
		{#if data.commit.message}
			<h1 class="text-lg font-semibold text-text-primary whitespace-pre-wrap">
				{data.commit.message}
			</h1>
		{:else}
			<h1 class="text-lg font-semibold text-text-secondary italic">No commit message</h1>
		{/if}

		<div class="mt-3 flex flex-wrap items-center gap-x-4 gap-y-1 text-sm text-text-secondary">
			{#if data.commit.author?.name}
				<span>
					Author: <span class="font-mono text-text-primary">{data.commit.author.name}</span>
				</span>
			{/if}
			{#if data.commit.committer?.name && data.commit.committer.name !== data.commit.author?.name}
				<span>
					Committer: <span class="font-mono text-text-primary">{data.commit.committer.name}</span>
				</span>
			{/if}
			{#if data.commit.timestamp}
				<time>{formatTime(data.commit.timestamp)}</time>
			{/if}
		</div>

		<div class="mt-3 flex items-center gap-2">
			<span class="text-xs text-text-secondary">Commit</span>
			<code class="rounded bg-surface-2 px-2 py-0.5 font-mono text-xs text-text-primary break-all">
				{data.commit.oid}
			</code>
		</div>

		{#if data.commit.parents.length > 0}
			<div class="mt-3 flex flex-wrap items-center gap-2 text-xs text-text-secondary">
				<span>{data.commit.parents.length === 1 ? 'Parent:' : 'Parents:'}</span>
				{#each data.commit.parents as parent (parent)}
					<a
						href="{basePath}/commit/{parent}"
						class="font-mono text-accent transition-colors hover:text-accent-hover"
					>
						{truncateHash(parent)}
					</a>
				{/each}
			</div>
		{/if}
	</div>

	<!-- Structural diff against first parent -->
	{#if data.diff}
		<div class="mt-5">
			<StructuralDiff diff={data.diff} />
		</div>
	{:else if data.commit.parents.length === 0}
		<div class="mt-5 rounded-md border border-border bg-surface-1 p-4 text-center text-sm text-text-secondary">
			Root commit — no parent to diff against.
		</div>
	{:else}
		<div class="mt-5 rounded-md border border-border bg-surface-1 p-4 text-center text-sm text-text-secondary">
			Diff unavailable.
		</div>
	{/if}
{:else}
	<div class="flex flex-col items-center gap-3 py-12 text-text-secondary">
		<svg class="h-12 w-12" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
			<path stroke-linecap="round" stroke-linejoin="round" d="M21 21l-5.197-5.197m0 0A7.5 7.5 0 105.196 5.196a7.5 7.5 0 0010.607 10.607z" />
		</svg>
		<p class="text-sm">Commit not found.</p>
		<p class="text-xs">
			Commit <code class="font-mono">{data.commitId}</code> could not be loaded from the node.
		</p>
		{#if data.error}
			<p class="text-xs text-breaking">{data.error}</p>
		{/if}
	</div>
{/if}

<BackLink href={basePath} />
