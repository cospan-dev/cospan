<script lang="ts">
	import { goto } from '$app/navigation';

	let { basePath }: { basePath: string } = $props();

	let waitingForSecondKey = $state(false);
	let firstKey = $state('');
	let timeout: ReturnType<typeof setTimeout> | undefined;

	function handleKeydown(event: KeyboardEvent) {
		const target = event.target as HTMLElement;
		if (
			target.tagName === 'INPUT' ||
			target.tagName === 'TEXTAREA' ||
			target.isContentEditable
		) {
			return;
		}

		if (waitingForSecondKey) {
			clearTimeout(timeout);
			waitingForSecondKey = false;

			if (firstKey === 'g') {
				if (event.key === 'i') {
					event.preventDefault();
					goto(`${basePath}/issues`);
					return;
				}
				if (event.key === 'p') {
					event.preventDefault();
					goto(`${basePath}/pulls`);
					return;
				}
				if (event.key === 'c') {
					event.preventDefault();
					goto(`${basePath}/tree`);
					return;
				}
			}

			firstKey = '';
			return;
		}

		if (event.key === 'g') {
			firstKey = 'g';
			waitingForSecondKey = true;
			timeout = setTimeout(() => {
				waitingForSecondKey = false;
				firstKey = '';
			}, 500);
		}
	}
</script>

<svelte:window onkeydown={handleKeydown} />
