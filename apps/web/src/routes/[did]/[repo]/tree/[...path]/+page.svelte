<script lang="ts">
	import FileTree from '$lib/components/repo/FileTree.svelte';
	import CodeView from '$lib/components/repo/CodeView.svelte';
	import Breadcrumb from '$lib/components/shared/Breadcrumb.svelte';

	let { data } = $props();

	let basePath = $derived(`/${data.repo.did}/${data.repo.name}`);

	let crumbs = $derived.by(() => {
		const result: { label: string; href?: string }[] = [
			{ label: data.repo.did, href: `/${data.repo.did}` },
			{ label: data.repo.name, href: basePath }
		];

		if (data.path) {
			const segments = data.path.split('/').filter(Boolean);
			let accumulated = '';
			for (let i = 0; i < segments.length; i++) {
				accumulated += (accumulated ? '/' : '') + segments[i];
				if (i < segments.length - 1) {
					result.push({ label: segments[i], href: `${basePath}/tree/${accumulated}` });
				} else {
					result.push({ label: segments[i] });
				}
			}
		} else {
			result.push({ label: 'Code' });
		}

		return result;
	});
</script>

<svelte:head>
	<title>{data.path || 'Code'} - {data.repo.name} - Cospan</title>
</svelte:head>

<section>
	<!-- Breadcrumb -->
	<div class="mb-4">
		<Breadcrumb {crumbs} />
	</div>

	<!-- Repo nav tabs -->
	<div class="mb-6 flex items-center gap-1 border-b border-border">
		<a
			href="{basePath}/tree"
			class="border-b-2 border-accent px-4 py-2 text-sm font-medium text-text-primary"
		>
			Code
		</a>
		<a
			href="{basePath}/issues"
			class="border-b-2 border-transparent px-4 py-2 text-sm text-text-secondary hover:text-text-primary transition-colors"
		>
			Issues
			{#if data.repo.openIssueCount > 0}
				<span class="ml-1 rounded-full bg-surface-2 px-1.5 py-0.5 text-xs">
					{data.repo.openIssueCount}
				</span>
			{/if}
		</a>
		<a
			href="{basePath}/pulls"
			class="border-b-2 border-transparent px-4 py-2 text-sm text-text-secondary hover:text-text-primary transition-colors"
		>
			Merge Requests
			{#if data.repo.openMrCount > 0}
				<span class="ml-1 rounded-full bg-surface-2 px-1.5 py-0.5 text-xs">
					{data.repo.openMrCount}
				</span>
			{/if}
		</a>
	</div>

	<!-- Error message -->
	{#if data.error}
		<div class="mb-4 rounded-lg border border-conflict/30 bg-conflict/5 px-4 py-3 text-sm text-conflict">
			{data.error}
		</div>
	{/if}

	<!-- Content -->
	{#if data.mode === 'blob' && data.object}
		<CodeView
			code={data.object.code}
			language={data.object.language}
			filePath={data.path}
			highlightedHtml={data.object.highlightedHtml}
		/>
	{:else}
		<FileTree refs={data.refs ?? []} {basePath} />
	{/if}

	<!-- Back to repo link -->
	<div class="mt-6">
		<a
			href={basePath}
			class="inline-flex items-center gap-1.5 text-sm text-accent transition-colors hover:text-accent-hover"
		>
			<svg class="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
				<path stroke-linecap="round" stroke-linejoin="round" d="M10.5 19.5L3 12m0 0l7.5-7.5M3 12h18" />
			</svg>
			Back to repository
		</a>
	</div>
</section>
