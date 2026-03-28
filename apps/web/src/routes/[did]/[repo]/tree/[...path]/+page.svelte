<script lang="ts">
	import FileTree from '$lib/components/repo/FileTree.svelte';
	import CodeView from '$lib/components/repo/CodeView.svelte';
	import RepoTabBar from '$lib/components/repo/RepoTabBar.svelte';
	import Breadcrumb from '$lib/components/shared/Breadcrumb.svelte';
	import BackLink from '$lib/components/shared/BackLink.svelte';

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
	<RepoTabBar
		{basePath}
		activeTab="code"
		openIssueCount={data.repo.openIssueCount}
		openMrCount={data.repo.openMrCount}
	/>

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
	<BackLink href={basePath} />
</section>
