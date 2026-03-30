<script lang="ts">
	import { page } from '$app/stores';
	import { setContext } from 'svelte';
	import { getAuth } from '$lib/stores/auth.svelte';
	import Breadcrumb from '$lib/components/shared/Breadcrumb.svelte';
	import RepoTabBar from '$lib/components/repo/RepoTabBar.svelte';
	import KeyboardShortcuts from '$lib/components/shared/KeyboardShortcuts.svelte';

	let { data, children } = $props();

	let auth = $derived(getAuth());
	let isOwner = $derived(auth.authenticated && data.repo?.did === auth.did);
	let basePath = $derived(data.repo
		? `/${data.repo.did}/${data.repo.name}`
		: `/${data.did}/${data.repoName}`
	);
	let isTangled = $derived(data.repo?.source === 'tangled');

	// Auto-detect activeTab from route
	let activeTab = $derived.by(() => {
		const routeId = $page.route.id ?? '';
		if (routeId.includes('/issues')) return 'issues';
		if (routeId.includes('/pulls')) return 'pulls';
		if (routeId.includes('/branches')) return 'branches';
		if (routeId.includes('/tags')) return 'tags';
		if (routeId.includes('/releases')) return 'releases';
		if (routeId.includes('/compare')) return 'compare';
		if (routeId.includes('/settings')) return 'settings';
		if (routeId.includes('/tree')) return 'code';
		if (routeId.includes('/commit')) return 'code';
		return 'code';
	});

	// Routes that should NOT show the tab bar
	const NO_TABS_SUFFIXES = ['/fork', '/new'];
	let showTabs = $derived(() => {
		const routeId = $page.route.id ?? '';
		return !NO_TABS_SUFFIXES.some(s => routeId.endsWith(s));
	});

	// Extra breadcrumb segments — child pages can extend via context
	let extraCrumbs = $state<{ label: string; href?: string }[]>([]);

	setContext('repoLayout', {
		setExtraCrumbs: (crumbs: { label: string; href?: string }[]) => {
			extraCrumbs = crumbs;
		},
		basePath: () => basePath,
		isTangled: () => isTangled,
	});

	let crumbs = $derived([
		{ label: data.did, href: `/${data.did}` },
		{ label: data.repoName, href: basePath },
		...extraCrumbs,
	]);
</script>

<KeyboardShortcuts {basePath} />

<section>
	<div class="mb-4">
		<Breadcrumb {crumbs} />
	</div>

	{#if showTabs() && data.repo}
		<RepoTabBar
			{basePath}
			{activeTab}
			openIssueCount={data.repo.openIssueCount}
			openMrCount={data.repo.openMrCount}
			{isOwner}
			{isTangled}
		/>
	{/if}

	{@render children()}
</section>
