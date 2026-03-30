<script lang="ts">
	import { getContext } from 'svelte';
	import FileTree from '$lib/components/repo/FileTree.svelte';
	import CodeView from '$lib/components/repo/CodeView.svelte';
	import BackLink from '$lib/components/shared/BackLink.svelte';

	let { data } = $props();

	let basePath = $derived(`/${data.repo.did}/${data.repo.name}`);

	const repoLayout = getContext<any>('repoLayout');
	$effect(() => {
		const crumbs: { label: string; href?: string }[] = [];

		if (data.path) {
			const segments = data.path.split('/').filter(Boolean);
			let accumulated = '';
			for (let i = 0; i < segments.length; i++) {
				accumulated += (accumulated ? '/' : '') + segments[i];
				if (i < segments.length - 1) {
					crumbs.push({ label: segments[i], href: `${basePath}/tree/${accumulated}` });
				} else {
					crumbs.push({ label: segments[i] });
				}
			}
		} else {
			crumbs.push({ label: 'Code' });
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
