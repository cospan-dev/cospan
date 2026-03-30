<script lang="ts">
	import StateBadge from '$lib/components/shared/StateBadge.svelte';
	import LabelBadge from '$lib/components/shared/LabelBadge.svelte';
	import Timeline from '$lib/components/shared/Timeline.svelte';
	import RepoTabBar from '$lib/components/repo/RepoTabBar.svelte';
	import Breadcrumb from '$lib/components/shared/Breadcrumb.svelte';
	import BackLink from '$lib/components/shared/BackLink.svelte';
	import KeyboardShortcuts from '$lib/components/shared/KeyboardShortcuts.svelte';
	import { timeAgo } from '$lib/utils/time.js';

	let { data } = $props();

	let basePath = $derived(`/${data.did}/${data.repo}`);

	let crumbs = $derived([
		{ label: data.did, href: `/${data.did}` },
		{ label: data.repo, href: basePath },
		{ label: 'Issues', href: `${basePath}/issues` },
		{ label: `#${data.issue.rkey}` }
	]);
</script>

<svelte:head>
	<title>{data.issue.title}· Issues · {data.repo} · Cospan</title>
</svelte:head>

<KeyboardShortcuts {basePath} />

<section>
	<Breadcrumb {crumbs} />

	<div class="mt-3 mb-6">
		<RepoTabBar {basePath} activeTab="issues" />

		<div class="flex flex-col gap-2 sm:flex-row sm:items-start sm:gap-3">
			<h1 class="text-2xl font-semibold text-text-primary">{data.issue.title}</h1>
			<div class="shrink-0">
				<StateBadge state={data.issue.state} />
			</div>
		</div>

		<div class="mt-2 flex flex-wrap items-center gap-3 text-sm text-text-secondary">
			<span>
				{data.issue.creatorHandle ?? data.issue.creatorDid} opened this {timeAgo(data.issue.createdAt)}
			</span>
			<span>{data.issue.commentCount} comments</span>
		</div>

		{#if data.issue.labels.length > 0}
			<div class="mt-3 flex flex-wrap gap-1">
				{#each data.issue.labels as label}
					<LabelBadge name={label} />
				{/each}
			</div>
		{/if}
	</div>

	{#if data.issue.body}
		<div class="mb-6 rounded-lg border border-border bg-surface-1">
			<div class="flex items-center gap-2 border-b border-border px-4 py-2">
				<span class="text-sm font-medium text-text-primary">
					{data.issue.creatorHandle ?? data.issue.creatorDid}
				</span>
				<span class="text-xs text-text-secondary">
					opened {timeAgo(data.issue.createdAt)}
				</span>
			</div>
			<div class="px-4 py-3 text-sm text-text-primary whitespace-pre-wrap">
				{data.issue.body}
			</div>
		</div>
	{/if}

	<Timeline events={data.timeline.events} />

	<BackLink href="{basePath}/issues" label="Back to issues" />
</section>
