<script lang="ts">
	import { getAuth } from '$lib/stores/auth.svelte';
	import { goto } from '$app/navigation';

	let auth = $derived(getAuth());

	let name = $state('');
	let description = $state('');
	let protocol = $state('typescript');
	let visibility = $state('public');
	let creating = $state(false);
	let error = $state('');

	const protocols = [
		{ value: 'typescript', label: 'TypeScript' },
		{ value: 'python', label: 'Python' },
		{ value: 'rust', label: 'Rust' },
		{ value: 'java', label: 'Java' },
		{ value: 'go', label: 'Go' },
		{ value: 'swift', label: 'Swift' },
		{ value: 'kotlin', label: 'Kotlin' },
		{ value: 'csharp', label: 'C#' },
		{ value: 'protobuf', label: 'Protocol Buffers' },
		{ value: 'graphql', label: 'GraphQL' },
		{ value: 'json_schema', label: 'JSON Schema' },
		{ value: 'sql', label: 'SQL' },
		{ value: 'atproto', label: 'ATProto Lexicon' },
	];

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
					protocol,
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
					<label for="protocol" class="mb-1 block text-xs font-medium text-text-secondary">
						Protocol
					</label>
					<select
						id="protocol"
						bind:value={protocol}
						class="w-full rounded-md border border-border bg-surface-0 px-3 py-2 text-sm text-text-primary focus:border-accent focus:outline-none"
					>
						{#each protocols as p}
							<option value={p.value}>{p.label}</option>
						{/each}
					</select>
					<p class="mt-1 text-xs text-text-secondary">The schema protocol this repository tracks. Determines structural diff and merge behavior.</p>
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
