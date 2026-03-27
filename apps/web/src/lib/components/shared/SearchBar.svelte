<script lang="ts">
	import { debounce } from '$lib/utils/debounce.js';

	let {
		value = '',
		placeholder = 'Search...',
		onSearch
	}: {
		value?: string;
		placeholder?: string;
		onSearch?: (query: string) => void;
	} = $props();

	let inputValue = $state(value);

	const debouncedSearch = debounce((q: string) => {
		onSearch?.(q);
	}, 300);

	function handleInput(event: Event) {
		const target = event.target as HTMLInputElement;
		inputValue = target.value;
		debouncedSearch(inputValue);
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter') {
			onSearch?.(inputValue);
		}
	}
</script>

<div class="relative">
	<svg
		class="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-text-secondary"
		fill="none"
		viewBox="0 0 24 24"
		stroke="currentColor"
		stroke-width="2"
	>
		<path
			stroke-linecap="round"
			stroke-linejoin="round"
			d="M21 21l-5.197-5.197m0 0A7.5 7.5 0 105.196 5.196a7.5 7.5 0 0010.607 10.607z"
		/>
	</svg>
	<input
		type="text"
		value={inputValue}
		oninput={handleInput}
		onkeydown={handleKeydown}
		{placeholder}
		class="w-full rounded-lg border border-border bg-surface-1 py-2 pl-10 pr-4 text-sm text-text-primary placeholder:text-text-secondary focus:border-accent focus:outline-none"
	/>
</div>
