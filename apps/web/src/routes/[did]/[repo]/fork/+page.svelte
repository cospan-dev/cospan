<script lang="ts">
	import { goto } from '$app/navigation';
	import { getAuth } from '$lib/stores/auth.svelte';
	import Breadcrumb from '$lib/components/shared/Breadcrumb.svelte';
	import BackLink from '$lib/components/shared/BackLink.svelte';

	let { data } = $props();

	let auth = $derived(getAuth());
	let forkName = $state(data.repoName);
	let submitting = $state(false);
	let error = $state('');

	let crumbs = $derived([
		{ label: data.did, href: `/${data.did}` },
		{ label: data.repoName, href: `/${data.did}/${data.repoName}` },
		{ label: 'Fork' }
	]);

	async function handleFork() {
		if (!auth.authenticated || !forkName.trim() || submitting) return;
		submitting = true;
		error = '';

		try {
			const resp = await fetch('/xrpc/dev.cospan.repo.fork', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					sourceDid: data.did,
					sourceRepo: data.repoName,
					name: forkName.trim(),
				}),
			});

			if (!resp.ok) {
				const body = await resp.json().catch(() => ({}));
				error = body.message ?? `Fork failed (${resp.status})`;
				return;
			}

			const result = await resp.json();
			const targetDid = result.did ?? auth.did;
			const targetName = result.name ?? forkName.trim();
			goto(`/${targetDid}/${targetName}`);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Fork failed. Please try again.';
		} finally {
			submitting = false;
		}
	}
</script>

<svelte:head>
	<title>Fork {data.repoName} - Cospan</title>
</svelte:head>

<section class="mx-auto max-w-xl">
	<Breadcrumb {crumbs} />

	<h1 class="mt-4 text-xl font-semibold text-text-primary">Fork repository</h1>
	<p class="mt-1 text-sm text-text-secondary">
		Create a copy of <span class="font-mono text-text-primary">{data.did}/{data.repoName}</span> under your account.
	</p>

	<div class="mt-6 rounded-lg border border-border bg-surface-1 p-5">
		<div class="mb-5">
			<h3 class="text-sm font-medium text-text-primary">Source</h3>
			<div class="mt-2 flex items-center gap-2">
				<span class="rounded bg-surface-2 px-2 py-1 font-mono text-xs text-text-secondary">
					{data.did}
				</span>
				<span class="text-text-secondary">/</span>
				<span class="font-mono text-sm text-text-primary">{data.repoName}</span>
			</div>
			{#if data.protocol}
				<span class="mt-2 inline-block rounded-full bg-surface-2 px-2 py-0.5 font-mono text-xs text-text-secondary">
					{data.protocol}
				</span>
			{/if}
		</div>

		<div class="mb-5 border-t border-border pt-5">
			<h3 class="text-sm font-medium text-text-primary">Destination</h3>
			<div class="mt-2 flex items-center gap-2">
				<span class="rounded bg-surface-2 px-2 py-1 font-mono text-xs text-accent">
					{auth.did ?? 'your-did'}
				</span>
				<span class="text-text-secondary">/</span>
				<input
					type="text"
					bind:value={forkName}
					placeholder="repository name"
					class="w-full rounded-md border border-border bg-surface-0 px-3 py-1.5 font-mono text-sm text-text-primary placeholder:text-text-secondary focus:border-accent focus:outline-none"
				/>
			</div>
		</div>

		{#if error}
			<div class="mb-4 rounded-md bg-breaking/10 px-3 py-2 text-sm text-breaking">
				{error}
			</div>
		{/if}

		<div class="flex items-center justify-end gap-3">
			<a
				href="/{data.did}/{data.repoName}"
				class="rounded-md px-4 py-2 text-sm text-text-secondary transition-colors hover:text-text-primary"
			>
				Cancel
			</a>
			<button
				onclick={handleFork}
				disabled={!auth.authenticated || !forkName.trim() || submitting}
				class="rounded-md bg-accent px-4 py-2 text-sm font-medium text-surface-0 transition-colors hover:bg-accent-hover disabled:opacity-50 disabled:cursor-default"
			>
				{submitting ? 'Forking...' : 'Create fork'}
			</button>
		</div>
	</div>

	<BackLink href="/{data.did}/{data.repoName}" />
</section>
