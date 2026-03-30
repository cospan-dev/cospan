<script lang="ts">
	import { onMount } from 'svelte';
	import { resolveHandle } from '$lib/api/handle.js';

	let { crumbs }: { crumbs: { label: string; href?: string }[] } = $props();

	// Auto-resolve any crumb label that looks like a DID
	let resolvedLabels = $state<Record<number, string>>({});

	onMount(async () => {
		const resolved: Record<number, string> = {};
		await Promise.allSettled(
			crumbs.map(async (crumb, i) => {
				if (crumb.label.startsWith('did:')) {
					resolved[i] = await resolveHandle(crumb.label);
				}
			})
		);
		resolvedLabels = resolved;
	});

	function getLabel(index: number): string {
		return resolvedLabels[index] || crumbs[index].label;
	}
</script>

<nav class="flex items-center gap-1.5 text-sm">
	{#each crumbs as crumb, i}
		{#if i > 0}
			<span class="text-ghost">/</span>
		{/if}
		{#if crumb.href && i < crumbs.length - 1}
			<a
				href={crumb.href}
				class="text-accent transition-colors hover:text-accent-hover"
			>
				{getLabel(i)}
			</a>
		{:else}
			<span class="text-ghost">{getLabel(i)}</span>
		{/if}
	{/each}
</nav>
