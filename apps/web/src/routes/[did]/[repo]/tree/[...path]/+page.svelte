<script lang="ts">
	import { getContext } from 'svelte';
	import CodeView from '$lib/components/repo/CodeView.svelte';
	import FileSchemaSidebar from '$lib/components/repo/FileSchemaSidebar.svelte';
	import BackLink from '$lib/components/shared/BackLink.svelte';

	let { data } = $props();

	let basePath = $derived(`/${data.repo.did}/${data.repo.name}`);

	// Build the tree URL prefix, including the ref if present
	let treeBase = $derived(
		data.ref ? `${basePath}/tree/${data.ref}` : `${basePath}/tree`
	);

	const repoLayout = getContext<any>('repoLayout');
	$effect(() => {
		const crumbs: { label: string; href?: string }[] = [];
		crumbs.push({ label: 'Code', href: `${basePath}/tree` });

		if (data.path) {
			// Strip the ref prefix from the path for breadcrumb display
			const displayPath = data.ref && data.path.startsWith(data.ref)
				? data.path.slice(data.ref.length + 1)
				: data.path;
			if (displayPath) {
				const segments = displayPath.split('/').filter(Boolean);
				let accumulated = '';
				for (let i = 0; i < segments.length; i++) {
					accumulated += (accumulated ? '/' : '') + segments[i];
					if (i < segments.length - 1) {
						crumbs.push({ label: segments[i], href: `${treeBase}/${accumulated}` });
					} else {
						crumbs.push({ label: segments[i] });
					}
				}
			}
		}

		repoLayout?.setExtraCrumbs(crumbs);
		return () => repoLayout?.setExtraCrumbs([]);
	});
</script>

<svelte:head>
	<title>{data.path || 'Code'} · {data.repo.name} · Cospan</title>
</svelte:head>

<!-- Error message -->
{#if data.error}
	<div class="mb-4 rounded-lg border border-conflict/30 bg-conflict/5 px-4 py-3 text-sm text-conflict">
		{data.error}
	</div>
{/if}

<!-- Content -->
{#if data.mode === 'blob' && data.object}
	<div class="flex items-start gap-4">
		<div class="min-w-0 flex-1">
			<CodeView
				code={data.object.code}
				language={data.object.language}
				filePath={data.path}
				highlightedHtml={data.object.highlightedHtml}
			/>
		</div>
		{#if data.fileSchema}
			<div class="hidden w-64 shrink-0 lg:block" style="position: sticky; top: 1rem;">
				<FileSchemaSidebar fileSchema={data.fileSchema} />
			</div>
		{/if}
	</div>
{:else if data.entries && data.entries.length > 0}
	<!-- Directory listing -->
	<div class="rounded-lg border border-border bg-surface-1">
		{#each data.entries as entry (entry.name)}
			<a
				href="{treeBase}/{data.path && !data.path.startsWith('refs/') ? data.path + '/' : ''}{entry.name}"
				class="flex items-center gap-3 border-b border-border/50 px-4 py-2.5 last:border-b-0 text-sm transition-colors hover:bg-surface-2/50"
			>
				{#if entry.type === 'dir'}
					<svg class="h-4 w-4 shrink-0 text-accent" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
						<path stroke-linecap="round" stroke-linejoin="round" d="M2.25 12.75V12A2.25 2.25 0 014.5 9.75h15A2.25 2.25 0 0121.75 12v.75m-8.69-6.44l-2.12-2.12a1.5 1.5 0 00-1.061-.44H4.5A2.25 2.25 0 002.25 6v12a2.25 2.25 0 002.25 2.25h15A2.25 2.25 0 0021.75 18V9a2.25 2.25 0 00-2.25-2.25h-5.379a1.5 1.5 0 01-1.06-.44z" />
					</svg>
				{:else}
					<svg class="h-4 w-4 shrink-0 text-text-muted" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
						<path stroke-linecap="round" stroke-linejoin="round" d="M19.5 14.25v-2.625a3.375 3.375 0 00-3.375-3.375h-1.5A1.125 1.125 0 0113.5 7.125v-1.5a3.375 3.375 0 00-3.375-3.375H8.25m2.25 0H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 00-9-9z" />
					</svg>
				{/if}
				<span class="font-mono text-xs text-text-primary">{entry.name}</span>
				{#if entry.type === 'file' && entry.size != null}
					<span class="ml-auto text-[10px] text-text-muted">
						{entry.size > 1024 ? `${(entry.size / 1024).toFixed(1)} KB` : `${entry.size} B`}
					</span>
				{/if}
			</a>
		{/each}
	</div>
{:else if !data.error}
	<div class="flex flex-col items-center gap-3 py-12 text-text-muted">
		<svg class="h-12 w-12" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
			<path stroke-linecap="round" stroke-linejoin="round" d="M2.25 12.75V12A2.25 2.25 0 014.5 9.75h15A2.25 2.25 0 0121.75 12v.75m-8.69-6.44l-2.12-2.12a1.5 1.5 0 00-1.061-.44H4.5A2.25 2.25 0 002.25 6v12a2.25 2.25 0 002.25 2.25h15A2.25 2.25 0 0021.75 18V9a2.25 2.25 0 00-2.25-2.25h-5.379a1.5 1.5 0 01-1.06-.44z" />
		</svg>
		<p class="text-sm">This directory is empty.</p>
	</div>
{/if}

<BackLink href={basePath} />
