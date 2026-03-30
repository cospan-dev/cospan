<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { getAuth } from '$lib/stores/auth.svelte';
	import Breadcrumb from '$lib/components/shared/Breadcrumb.svelte';
	import BackLink from '$lib/components/shared/BackLink.svelte';

	let auth = $derived(getAuth());
	let did = $derived($page.params.did);
	let repo = $derived($page.params.repo);
	let basePath = $derived(`/${did}/${repo}`);

	let title = $state('');
	let body = $state('');
	let creating = $state(false);
	let error = $state('');

	let crumbs = $derived([
		{ label: did, href: `/${did}` },
		{ label: repo, href: basePath },
		{ label: 'Issues', href: `${basePath}/issues` },
		{ label: 'New' },
	]);

	async function handleSubmit(event: Event) {
		event.preventDefault();
		if (!title.trim() || !auth.did || creating) return;

		creating = true;
		error = '';

		try {
			const resp = await fetch('/xrpc/dev.cospan.repo.issue.create', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					repo: `at://${did}/dev.cospan.repo/${repo}`,
					title: title.trim(),
					body: body.trim() || undefined,
				}),
			});

			if (resp.ok) {
				goto(`${basePath}/issues`);
			} else {
				const data = await resp.json().catch(() => ({}));
				error = data.message ?? 'Failed to create issue';
			}
		} catch {
			error = 'Network error';
		} finally {
			creating = false;
		}
	}
</script>

<svelte:head>
	<title>New Issue · {repo} · Cospan</title>
</svelte:head>

<section class="mx-auto max-w-3xl">
	<Breadcrumb {crumbs} />

	<h1 class="mt-4 mb-6 text-xl font-semibold text-text-primary">New Issue</h1>

	{#if !auth.authenticated}
		<div class="rounded-lg border border-border bg-surface-1 p-8 text-center">
			<p class="text-text-secondary">Sign in to create an issue.</p>
		</div>
	{:else}
		<form onsubmit={handleSubmit} class="space-y-4">
			<div>
				<input
					bind:value={title}
					type="text"
					required
					placeholder="Issue title"
					class="w-full rounded-md border border-border bg-surface-1 px-4 py-3 text-sm text-text-primary placeholder:text-text-secondary focus:border-accent focus:outline-none"
				/>
			</div>

			<div>
				<textarea
					bind:value={body}
					rows="10"
					placeholder="Describe the issue... (Markdown supported)"
					class="w-full rounded-md border border-border bg-surface-1 px-4 py-3 text-sm text-text-primary placeholder:text-text-secondary focus:border-accent focus:outline-none resize-y"
				></textarea>
			</div>

			{#if error}
				<p class="text-sm text-breaking">{error}</p>
			{/if}

			<div class="flex items-center gap-3">
				<button
					type="submit"
					disabled={creating || !title.trim()}
					class="rounded-md bg-accent px-5 py-2 text-sm font-medium text-white transition-colors hover:bg-accent-hover disabled:opacity-50"
				>
					{creating ? 'Creating...' : 'Submit new issue'}
				</button>
				<a
					href="{basePath}/issues"
					class="text-sm text-text-secondary hover:text-text-primary transition-colors"
				>
					Cancel
				</a>
			</div>
		</form>
	{/if}

	<BackLink href="{basePath}/issues" label="Back to issues" />
</section>
