<script lang="ts">
	import { login } from '$lib/auth/oauth-client';

	let showInput = $state(false);
	let handle = $state('');
	let inputRef = $state<HTMLInputElement | null>(null);
	let suggestions = $state<{ handle: string; displayName: string | null; avatar: string | null }[]>([]);
	let selectedIndex = $state(-1);
	let showSuggestions = $state(false);
	let debounceTimer: ReturnType<typeof setTimeout> | null = null;
	let loading = $state(false);
	let signingIn = $state(false);

	function toggleInput() {
		showInput = !showInput;
		if (showInput) {
			setTimeout(() => inputRef?.focus(), 0);
		} else {
			reset();
		}
	}

	function reset() {
		suggestions = [];
		showSuggestions = false;
		selectedIndex = -1;
		loading = false;
	}

	function handleInput() {
		const q = handle.trim();
		selectedIndex = -1;

		if (q.length < 2) {
			reset();
			return;
		}

		if (debounceTimer) clearTimeout(debounceTimer);
		loading = true;
		debounceTimer = setTimeout(() => searchHandles(q), 200);
	}

	async function searchHandles(q: string) {
		try {
			const resp = await fetch(
				`https://public.api.bsky.app/xrpc/app.bsky.actor.searchActorsTypeahead?q=${encodeURIComponent(q)}&limit=6`
			);
			if (!resp.ok) { reset(); return; }
			const data = await resp.json();
			suggestions = (data.actors ?? []).map((a: any) => ({
				handle: a.handle,
				displayName: a.displayName ?? null,
				avatar: a.avatar ?? null,
			}));
			showSuggestions = suggestions.length > 0;
		} catch {
			reset();
		} finally {
			loading = false;
		}
	}

	function selectSuggestion(h: string) {
		handle = h;
		showSuggestions = false;
		suggestions = [];
		inputRef?.focus();
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			if (showSuggestions) { showSuggestions = false; return; }
			showInput = false; handle = ''; reset();
		} else if (event.key === 'ArrowDown' && showSuggestions) {
			event.preventDefault();
			selectedIndex = Math.min(selectedIndex + 1, suggestions.length - 1);
		} else if (event.key === 'ArrowUp' && showSuggestions) {
			event.preventDefault();
			selectedIndex = Math.max(selectedIndex - 1, -1);
		} else if ((event.key === 'Tab' || event.key === 'Enter') && showSuggestions && selectedIndex >= 0) {
			event.preventDefault();
			selectSuggestion(suggestions[selectedIndex].handle);
		}
	}

	async function handleSubmit(event: Event) {
		event.preventDefault();
		const trimmed = handle.trim();
		if (!trimmed || signingIn) return;
		signingIn = true;
		showSuggestions = false;
		try {
			await login(trimmed);
		} catch (e) {
			console.error('Login failed:', e);
			signingIn = false;
		}
	}
</script>

{#if showInput}
	<form onsubmit={handleSubmit} class="relative flex items-center gap-2">
		<div class="relative">
			<input
				bind:this={inputRef}
				bind:value={handle}
				oninput={handleInput}
				onkeydown={handleKeydown}
				onfocus={() => { if (suggestions.length > 0) showSuggestions = true; }}
				onblur={() => setTimeout(() => { showSuggestions = false; }, 200)}
				type="text"
				placeholder="Search for your handle..."
				autocomplete="off"
				spellcheck="false"
				disabled={signingIn}
				class="w-56 rounded-md border border-border bg-surface-0 px-2.5 py-1.5 text-xs text-text-primary placeholder:text-text-secondary focus:border-accent focus:outline-none disabled:opacity-50"
			/>
			{#if loading && !showSuggestions}
				<div class="absolute right-2 top-1/2 -translate-y-1/2">
					<div class="h-3 w-3 animate-spin rounded-full border border-text-secondary border-t-transparent"></div>
				</div>
			{/if}
			{#if showSuggestions}
				<ul class="absolute left-0 top-full z-50 mt-1 w-72 overflow-hidden rounded-lg border border-border bg-surface-1 shadow-xl">
					{#each suggestions as suggestion, i}
						<li>
							<button
								type="button"
								onmousedown={() => selectSuggestion(suggestion.handle)}
								class="flex w-full items-center gap-2.5 px-3 py-2 text-left transition-colors
									{i === selectedIndex ? 'bg-accent/15' : 'hover:bg-surface-2'}"
							>
								{#if suggestion.avatar}
									<img src={suggestion.avatar} alt="" class="h-6 w-6 shrink-0 rounded-full" />
								{:else}
									<div class="flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-surface-2 text-[10px] font-medium text-text-secondary">
										{suggestion.handle[0]?.toUpperCase() ?? '?'}
									</div>
								{/if}
								<div class="min-w-0 flex-1">
									{#if suggestion.displayName}
										<div class="truncate text-xs font-medium text-text-primary">{suggestion.displayName}</div>
									{/if}
									<div class="truncate text-xs text-text-secondary">@{suggestion.handle}</div>
								</div>
							</button>
						</li>
					{/each}
				</ul>
			{/if}
		</div>
		<button type="submit" disabled={signingIn}
			class="rounded-md bg-accent px-3 py-1.5 text-xs font-medium text-white transition-colors hover:bg-accent-hover disabled:opacity-50">
			{signingIn ? 'Signing in...' : 'Sign in'}
		</button>
		<button type="button" onclick={() => { showInput = false; handle = ''; reset(); }}
			class="text-xs text-text-secondary hover:text-text-primary transition-colors">Cancel</button>
	</form>
{:else}
	<button onclick={toggleInput}
		class="rounded-md bg-accent px-3 py-1.5 text-xs font-medium text-white transition-colors hover:bg-accent-hover">
		Sign in with ATProto
	</button>
{/if}
