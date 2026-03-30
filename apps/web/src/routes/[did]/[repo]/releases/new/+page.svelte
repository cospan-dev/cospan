<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import { getContext } from 'svelte';
	import { getAuth } from '$lib/stores/auth.svelte';
	import { createRelease } from '$lib/api/release.js';

	let did = $derived($page.params.did);
	let repo = $derived($page.params.repo);
	let basePath = $derived(`/${did}/${repo}`);
	let auth = $derived(getAuth());

	let tag = $state('');
	let title = $state('');
	let body = $state('');
	let draft = $state(false);
	let prerelease = $state(false);
	let submitting = $state(false);
	let error = $state('');

	const repoLayout = getContext<any>('repoLayout');
	$effect(() => {
		repoLayout?.setExtraCrumbs([
			{ label: 'Releases', href: `${basePath}/releases` },
			{ label: 'New' }
		]);
		return () => repoLayout?.setExtraCrumbs([]);
	});

	async function handleSubmit() {
		if (!tag.trim() || !title.trim() || submitting) return;
		submitting = true;
		error = '';

		try {
			const release = await createRelease({
				did,
				repo,
				tag: tag.trim(),
				title: title.trim(),
				body: body.trim() || undefined,
				draft,
				prerelease,
			});
			await goto(`${basePath}/releases/${release.tag}`);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to create release';
		} finally {
			submitting = false;
		}
	}
</script>

<svelte:head>
	<title>New Release · {repo} · Cospan</title>
</svelte:head>

<div class="mx-auto max-w-2xl">
	<div class="mb-6">
		<h1 class="mt-3 text-xl font-semibold text-text-primary">Create a new release</h1>
		<p class="mt-1 text-sm text-text-secondary">
			Releases bundle a tagged snapshot with release notes and downloadable artifacts.
		</p>
	</div>

	{#if !auth.authenticated}
		<div class="rounded-lg border border-border bg-surface-1 p-8 text-center">
			<p class="text-text-secondary">Sign in to create releases.</p>
		</div>
	{:else}
		<form onsubmit={handleSubmit} class="space-y-6">
			<div class="rounded-lg border border-border bg-surface-1 p-6 space-y-4">
				<div>
					<label for="release-tag" class="mb-1 block text-xs font-medium text-text-secondary">
						Tag name
					</label>
					<input
						id="release-tag"
						bind:value={tag}
						type="text"
						placeholder="v1.0.0"
						required
						class="w-full rounded-md border border-border bg-surface-0 px-3 py-2 font-mono text-sm text-text-primary placeholder:text-text-secondary focus:border-accent focus:outline-none"
					/>
					<p class="mt-1 text-xs text-text-secondary">
						Choose an existing tag or create a new one on publish.
					</p>
				</div>

				<div>
					<label for="release-title" class="mb-1 block text-xs font-medium text-text-secondary">
						Release title
					</label>
					<input
						id="release-title"
						bind:value={title}
						type="text"
						placeholder="Release title"
						required
						class="w-full rounded-md border border-border bg-surface-0 px-3 py-2 text-sm text-text-primary placeholder:text-text-secondary focus:border-accent focus:outline-none"
					/>
				</div>

				<div>
					<label for="release-body" class="mb-1 block text-xs font-medium text-text-secondary">
						Release notes
					</label>
					<textarea
						id="release-body"
						bind:value={body}
						rows="10"
						placeholder="Describe what's changed in this release..."
						class="w-full rounded-md border border-border bg-surface-0 px-3 py-2 text-sm text-text-primary placeholder:text-text-secondary focus:border-accent focus:outline-none resize-none"
					></textarea>
				</div>

				<div class="space-y-2">
					<label class="flex items-center gap-2 cursor-pointer">
						<input
							type="checkbox"
							bind:checked={prerelease}
							class="rounded border-border text-accent focus:ring-accent"
						/>
						<span class="text-sm text-text-primary">Pre-release</span>
						<span class="text-xs text-text-secondary">Mark as not ready for production</span>
					</label>

					<label class="flex items-center gap-2 cursor-pointer">
						<input
							type="checkbox"
							bind:checked={draft}
							class="rounded border-border text-accent focus:ring-accent"
						/>
						<span class="text-sm text-text-primary">Draft</span>
						<span class="text-xs text-text-secondary">Save without publishing</span>
					</label>
				</div>
			</div>

			{#if error}
				<p class="text-sm text-breaking">{error}</p>
			{/if}

			<div class="flex items-center gap-3">
				<button
					type="submit"
					disabled={submitting || !tag.trim() || !title.trim()}
					class="rounded-md bg-accent px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-accent-hover disabled:opacity-50"
				>
					{submitting ? 'Publishing...' : draft ? 'Save draft' : 'Publish release'}
				</button>
				<a
					href="{basePath}/releases"
					class="text-sm text-text-secondary transition-colors hover:text-text-primary"
				>
					Cancel
				</a>
			</div>
		</form>
	{/if}
</div>
