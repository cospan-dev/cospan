<script lang="ts">
	let { crumbs }: { crumbs: { label: string; href?: string }[] } = $props();
</script>

<nav class="flex items-center gap-1.5 text-sm">
	{#each crumbs as crumb, i}
		{#if i > 0}
			<span class="text-text-secondary">/</span>
		{/if}
		{#if crumb.href && i < crumbs.length - 1}
			<a
				href={crumb.href}
				class="text-accent transition-colors hover:text-accent-hover"
			>
				{crumb.label}
			</a>
		{:else}
			<span class="text-text-secondary">{crumb.label}</span>
		{/if}
	{/each}
</nav>
