<script lang="ts">
	let {
		basePath,
		activeTab = 'code',
		openIssueCount = 0,
		openMrCount = 0,
		isOwner = false,
		isTangled = false
	}: {
		basePath: string;
		activeTab?: string;
		openIssueCount?: number;
		openMrCount?: number;
		isOwner?: boolean;
		isTangled?: boolean;
	} = $props();

	let tabs = $derived(() => {
		if (isTangled) {
			// Tangled repos: only show tabs for data we actually have
			return [
				{ id: 'code', label: 'Overview', href: basePath, badge: 0 },
				{ id: 'issues', label: 'Issues', href: `${basePath}/issues`, badge: openIssueCount },
				{ id: 'pulls', label: 'Merge Requests', href: `${basePath}/pulls`, badge: openMrCount },
			];
		}
		return [
			{ id: 'code', label: 'Code', href: basePath, badge: 0 },
			{ id: 'issues', label: 'Issues', href: `${basePath}/issues`, badge: openIssueCount },
			{ id: 'pulls', label: 'Merge Requests', href: `${basePath}/pulls`, badge: openMrCount },
			{ id: 'branches', label: 'Branches', href: `${basePath}/branches`, badge: 0 },
			{ id: 'tags', label: 'Tags', href: `${basePath}/tags`, badge: 0 },
			{ id: 'releases', label: 'Releases', href: `${basePath}/releases`, badge: 0 },
			{ id: 'compare', label: 'Compare', href: `${basePath}/compare`, badge: 0 },
			...(isOwner ? [{ id: 'settings', label: 'Settings', href: `${basePath}/settings`, badge: 0 }] : []),
		];
	});
</script>

<div class="mb-6 flex items-center gap-1 overflow-x-auto border-b border-line scrollbar-none">
	{#each tabs() as tab (tab.id)}
		<a
			href={tab.href}
			class="whitespace-nowrap border-b-2 px-4 py-2 text-sm font-medium transition-colors
				{activeTab === tab.id
					? 'border-focus text-ink'
					: 'border-transparent text-ghost hover:text-caption'}"
		>
			{tab.label}
			{#if tab.badge > 0}
				<span class="ml-1 text-xs text-ghost">
					{tab.badge}
				</span>
			{/if}
		</a>
	{/each}
</div>
