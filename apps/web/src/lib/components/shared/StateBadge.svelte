<script lang="ts">
	let { state }: { state: 'open' | 'closed' | 'merged' } = $props();

	let label = $derived(state.charAt(0).toUpperCase() + state.slice(1));

	let colorClass = $derived(
		state === 'open'
			? 'text-success'
			: state === 'merged'
				? 'text-info'
				: 'text-text-muted'
	);
</script>

<span class="inline-flex items-center text-xs font-medium {colorClass}">
	{label}
</span>
