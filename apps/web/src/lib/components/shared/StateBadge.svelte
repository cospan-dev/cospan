<script lang="ts">
	let { state }: { state: 'open' | 'closed' | 'merged' } = $props();

	let label = $derived(state.charAt(0).toUpperCase() + state.slice(1));

	let colorClass = $derived(
		state === 'open'
			? 'bg-compatible/15 text-compatible'
			: state === 'merged'
				? 'bg-lens/15 text-lens'
				: 'bg-breaking/15 text-breaking'
	);
</script>

<span class="inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium {colorClass}">
	{label}
</span>
