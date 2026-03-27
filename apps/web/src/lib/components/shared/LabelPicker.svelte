<script lang="ts">
	import LabelBadge from './LabelBadge.svelte';

	export interface LabelOption {
		name: string;
		color: string;
		description?: string;
	}

	let {
		labels = [],
		selected = [],
		onchange,
	}: {
		labels: LabelOption[];
		selected: string[];
		onchange?: (selected: string[]) => void;
	} = $props();

	let open = $state(false);
	let filterText = $state('');

	let filteredLabels = $derived(
		labels.filter(
			(l) => l.name.toLowerCase().includes(filterText.toLowerCase())
		)
	);

	function toggle(name: string) {
		const idx = selected.indexOf(name);
		let next: string[];
		if (idx >= 0) {
			next = [...selected.slice(0, idx), ...selected.slice(idx + 1)];
		} else {
			next = [...selected, name];
		}
		onchange?.(next);
	}

	function isSelected(name: string): boolean {
		return selected.includes(name);
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			open = false;
		}
	}
</script>

<div class="relative">
	<button
		type="button"
		onclick={() => { open = !open; }}
		class="inline-flex items-center gap-1.5 rounded-md border border-border bg-surface-1 px-3 py-1.5 text-xs text-text-secondary transition-colors hover:border-accent hover:text-text-primary"
	>
		<svg class="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
			<path stroke-linecap="round" stroke-linejoin="round" d="M9.568 3H5.25A2.25 2.25 0 003 5.25v4.318c0 .597.237 1.17.659 1.591l9.581 9.581c.699.699 1.78.872 2.607.33a18.095 18.095 0 005.223-5.223c.542-.827.369-1.908-.33-2.607L11.16 3.66A2.25 2.25 0 009.568 3z" />
			<path stroke-linecap="round" stroke-linejoin="round" d="M6 6h.008v.008H6V6z" />
		</svg>
		Labels
		{#if selected.length > 0}
			<span class="rounded-full bg-accent/20 px-1.5 text-accent">{selected.length}</span>
		{/if}
	</button>

	{#if open}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="absolute left-0 top-full z-50 mt-1 w-64 rounded-lg border border-border bg-surface-1 shadow-lg"
			onkeydown={handleKeydown}
		>
			<div class="border-b border-border p-2">
				<input
					type="text"
					bind:value={filterText}
					placeholder="Filter labels..."
					class="w-full rounded-md border border-border bg-surface-0 px-2 py-1 text-xs text-text-primary placeholder:text-text-secondary focus:border-accent focus:outline-none"
				/>
			</div>

			<div class="max-h-48 overflow-y-auto p-1">
				{#if filteredLabels.length === 0}
					<p class="px-3 py-2 text-xs text-text-secondary">No labels found.</p>
				{:else}
					{#each filteredLabels as label (label.name)}
						<button
							type="button"
							onclick={() => toggle(label.name)}
							class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-xs transition-colors hover:bg-surface-2"
						>
							<span
								class="flex h-4 w-4 shrink-0 items-center justify-center rounded border {isSelected(label.name) ? 'border-accent bg-accent text-white' : 'border-border'}"
							>
								{#if isSelected(label.name)}
									<svg class="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="3">
										<path stroke-linecap="round" stroke-linejoin="round" d="M4.5 12.75l6 6 9-13.5" />
									</svg>
								{/if}
							</span>
							<LabelBadge name={label.name} color={label.color} />
							{#if label.description}
								<span class="truncate text-text-secondary">{label.description}</span>
							{/if}
						</button>
					{/each}
				{/if}
			</div>
		</div>

		<!-- Click-outside backdrop -->
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="fixed inset-0 z-40"
			onclick={() => { open = false; }}
			onkeydown={handleKeydown}
		></div>
	{/if}
</div>
