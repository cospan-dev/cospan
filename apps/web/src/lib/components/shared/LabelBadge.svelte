<script lang="ts">
	let { name, color = '#6b7280' }: { name: string; color?: string } = $props();

	/**
	 * Compute a readable foreground color from a hex background.
	 * Uses relative luminance to pick white or dark text.
	 */
	let foreground = $derived.by(() => {
		const hex = color.replace('#', '');
		const r = parseInt(hex.substring(0, 2), 16) / 255;
		const g = parseInt(hex.substring(2, 4), 16) / 255;
		const b = parseInt(hex.substring(4, 6), 16) / 255;
		const luminance = 0.2126 * r + 0.7152 * g + 0.0722 * b;
		return luminance > 0.4 ? '#0f0f14' : '#e8e8ed';
	});
</script>

<span
	class="inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium"
	style="background-color: {color}; color: {foreground}"
>
	{name}
</span>
