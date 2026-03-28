<script lang="ts">
	let { href, label = 'Back to repository' }: { href: string; label?: string } = $props();
</script>

<div class="mt-6">
	<a
		{href}
		class="inline-flex items-center gap-1.5 text-sm text-accent transition-colors hover:text-accent-hover"
	>
		<svg class="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
			<path stroke-linecap="round" stroke-linejoin="round" d="M10.5 19.5L3 12m0 0l7.5-7.5M3 12h18" />
		</svg>
		{label}
	</a>
</div>
