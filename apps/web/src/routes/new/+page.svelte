<script lang="ts">
	import { getAuth } from '$lib/stores/auth.svelte';
	import { goto } from '$app/navigation';
	import { ALL_LANGUAGES } from '$lib/data/languages';

	let auth = $derived(getAuth());

	let name = $state('');
	let description = $state('');
	let selectedProtocols = $state<string[]>(['typescript']);
	let protocolSearch = $state('');
	let showProtocolDropdown = $state(false);
	let visibility = $state('public');
	let creating = $state(false);
	let error = $state('');

	let filteredProtocols = $derived(
		protocolSearch.trim()
			? ALL_LANGUAGES.filter((p) =>
					p.label.toLowerCase().includes(protocolSearch.toLowerCase()) ||
					p.value.toLowerCase().includes(protocolSearch.toLowerCase())
				)
			: ALL_LANGUAGES
	);

	function toggleProtocol(value: string) {
		if (selectedProtocols.includes(value)) {
			selectedProtocols = selectedProtocols.filter((p) => p !== value);
		} else {
			selectedProtocols = [...selectedProtocols, value];
		}
	}

	function removeProtocol(value: string) {
		selectedProtocols = selectedProtocols.filter((p) => p !== value);
	}

	function protocolLabel(value: string): string {
		return ALL_LANGUAGES.find((p) => p.value === value)?.label ?? value;
	}

	async function handleCreate(event: Event) {
		event.preventDefault();
		if (!name.trim() || !auth.did || creating) return;

		creating = true;
		error = '';

		try {
			const resp = await fetch('/xrpc/dev.cospan.repo.create', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					did: auth.did,
					name: name.trim(),
					description: description.trim() || undefined,
					protocol: selectedProtocols.join(','),
					visibility,
				}),
			});

			if (resp.ok) {
				goto(`/${auth.did}/${name.trim()}`);
			} else {
				const body = await resp.json().catch(() => ({}));
				error = body.message ?? 'Failed to create repository';
			}
		} catch {
			error = 'Network error';
		} finally {
			creating = false;
		}
	}
</script>

<svelte:head>
	<title>New Repository - Cospan</title>
</svelte:head>

<section class="mx-auto max-w-2xl">
	<h1 class="mb-6 text-xl font-semibold text-text-primary">Create a new repository</h1>

	{#if !auth.authenticated}
		<div class="rounded-lg border border-border bg-surface-1 p-8 text-center">
			<p class="text-text-secondary">Sign in to create a repository.</p>
		</div>
	{:else}
		<form onsubmit={handleCreate} class="space-y-6">
			<div class="rounded-lg border border-border bg-surface-1 p-6 space-y-4">
				<div>
					<label for="name" class="mb-1 block text-xs font-medium text-text-secondary">
						Repository name <span class="text-breaking">*</span>
					</label>
					<input
						id="name"
						bind:value={name}
						type="text"
						required
						pattern="[a-zA-Z0-9_-]+"
						placeholder="my-project"
						class="w-full rounded-md border border-border bg-surface-0 px-3 py-2 text-sm text-text-primary font-mono placeholder:text-text-secondary focus:border-accent focus:outline-none"
					/>
					<p class="mt-1 text-xs text-text-secondary">Letters, numbers, hyphens, underscores only.</p>
				</div>

				<div>
					<label for="description" class="mb-1 block text-xs font-medium text-text-secondary">
						Description
					</label>
					<input
						id="description"
						bind:value={description}
						type="text"
						placeholder="A short description of your project"
						class="w-full rounded-md border border-border bg-surface-0 px-3 py-2 text-sm text-text-primary placeholder:text-text-secondary focus:border-accent focus:outline-none"
					/>
				</div>

				<div>
					<label class="mb-1 block text-xs font-medium text-text-secondary">
						Languages ({selectedProtocols.length} selected)
					</label>

					<!-- Selected tags -->
					{#if selectedProtocols.length > 0}
						<div class="mb-2 flex flex-wrap gap-1.5">
							{#each selectedProtocols as p}
								<button
									type="button"
									onclick={() => removeProtocol(p)}
									class="inline-flex items-center gap-1 rounded-full bg-accent/15 px-2.5 py-0.5 text-xs font-medium text-accent transition-colors hover:bg-accent/25"
								>
									{protocolLabel(p)}
									<svg class="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
										<path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
									</svg>
								</button>
							{/each}
						</div>
					{/if}

					<!-- Search input -->
					<div class="relative">
						<input
							type="text"
							bind:value={protocolSearch}
							onfocus={() => showProtocolDropdown = true}
							onblur={() => setTimeout(() => showProtocolDropdown = false, 200)}
							placeholder="Search 217 languages..."
							autocomplete="off"
							class="w-full rounded-md border border-border bg-surface-0 px-3 py-2 text-sm text-text-primary placeholder:text-text-secondary focus:border-accent focus:outline-none"
						/>

						{#if showProtocolDropdown}
							<ul class="absolute left-0 top-full z-50 mt-1 max-h-48 w-full overflow-y-auto rounded-lg border border-border bg-surface-1 shadow-lg">
								{#each filteredProtocols as p}
									<li>
										<button
											type="button"
											onmousedown={() => toggleProtocol(p.value)}
											class="flex w-full items-center gap-2 px-3 py-1.5 text-left text-xs transition-colors hover:bg-surface-2
												{selectedProtocols.includes(p.value) ? 'text-accent' : 'text-text-secondary'}"
										>
											{#if selectedProtocols.includes(p.value)}
												<svg class="h-3 w-3 shrink-0 text-accent" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="3">
													<path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7" />
												</svg>
											{:else}
												<span class="h-3 w-3 shrink-0"></span>
											{/if}
											{p.label}
										</button>
									</li>
								{/each}
								{#if filteredProtocols.length === 0}
									<li class="px-3 py-2 text-xs text-text-secondary">No matches</li>
								{/if}
							</ul>
						{/if}
					</div>

					<p class="mt-1 text-xs text-text-secondary">Languages this repository uses. Determines structural diff and merge behavior.</p>
				</div>

				<div>
					<label class="mb-1 block text-xs font-medium text-text-secondary">Visibility</label>
					<div class="flex gap-4">
						<label class="flex items-center gap-2 text-sm text-text-primary">
							<input type="radio" bind:group={visibility} value="public" class="accent-accent" />
							Public
						</label>
						<label class="flex items-center gap-2 text-sm text-text-secondary">
							<input type="radio" bind:group={visibility} value="unlisted" class="accent-accent" />
							Unlisted
						</label>
					</div>
				</div>
			</div>

			{#if error}
				<p class="text-sm text-breaking">{error}</p>
			{/if}

			<button
				type="submit"
				disabled={creating || !name.trim()}
				class="rounded-md bg-accent px-6 py-2.5 text-sm font-medium text-white transition-colors hover:bg-accent-hover disabled:opacity-50"
			>
				{creating ? 'Creating...' : 'Create repository'}
			</button>
		</form>

		<div class="mt-8 rounded-lg border border-border bg-surface-1 p-6">
			<h2 class="mb-3 text-sm font-medium text-text-primary">Or push an existing project</h2>
			<div class="space-y-3 rounded-md bg-surface-0 p-4 font-mono text-xs">
				<div>
					<p class="text-text-secondary mb-1"># Initialize a panproto repository</p>
					<p class="text-text-primary">schema init</p>
				</div>
				<div>
					<p class="text-text-secondary mb-1"># Add and commit your schema</p>
					<p class="text-text-primary">schema add .</p>
					<p class="text-text-primary">schema commit -m "initial commit"</p>
				</div>
				<div>
					<p class="text-text-secondary mb-1"># Add a remote and push</p>
					<p class="text-text-primary">schema remote add origin cospan://node.cospan.dev/{auth.did}/{name || 'my-project'}</p>
					<p class="text-text-primary">schema push origin main</p>
				</div>
			</div>
		</div>
	{/if}
</section>
