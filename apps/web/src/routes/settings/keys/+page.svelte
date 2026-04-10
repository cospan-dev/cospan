<script lang="ts">
	import { onMount } from 'svelte';
	import { getAuth } from '$lib/stores/auth.svelte';
	import KeyForm from '$lib/components/settings/KeyForm.svelte';
	import { listKeys, addKey, deleteKey, type Key, type KeyType } from '$lib/api/keys.js';
	import { enhance } from '$app/forms';
	import { formatDate } from '$lib/utils/time.js';

	let auth = $derived(getAuth());
	let keys = $state<Key[]>([]);
	let loading = $state(true);
	let deletingKey = $state<string | null>(null);
	let showForm = $state(false);
	let activeTab: KeyType | 'push' = $state('ssh');

	let filteredKeys = $derived(keys.filter((k) => k.type === activeTab));

	let { form } = $props();

	// Push token state
	let pushToken = $derived(form?.token as string | undefined);
	let pushTokenError = $derived(form?.error as string | undefined);
	let pushTokenCopied = $state(false);

	async function copyToken() {
		if (!pushToken) return;
		await navigator.clipboard.writeText(pushToken);
		pushTokenCopied = true;
		setTimeout(() => { pushTokenCopied = false; }, 2000);
	}

	onMount(async () => {
		if (!auth.authenticated || !auth.did) {
			loading = false;
			return;
		}

		try {
			const result = await listKeys({ did: auth.did, limit: 100 });
			keys = result.items;
		} catch {
			keys = [];
		} finally {
			loading = false;
		}
	});

	async function handleAddKey(data: { type: KeyType; title: string; publicKey: string }) {
		if (!auth.did) return;
		const key = await addKey({ did: auth.did, ...data });
		keys = [key, ...keys];
		showForm = false;
	}

	async function handleDelete(rkey: string) {
		if (!auth.did || deletingKey) return;
		deletingKey = rkey;

		try {
			await deleteKey({ did: auth.did, rkey });
			keys = keys.filter((k) => k.rkey !== rkey);
		} catch (e) {
			console.error('Failed to delete key:', e);
		} finally {
			deletingKey = null;
		}
	}

	function truncateFingerprint(fp: string): string {
		if (fp.length <= 47) return fp;
		return fp.slice(0, 20) + '...' + fp.slice(-20);
	}
</script>

<svelte:head>
	<title>SSH & GPG Keys· Settings · Cospan</title>
</svelte:head>

<section class="mx-auto max-w-2xl">
	<div class="mb-6 flex items-center justify-between">
		<div>
			<h1 class="text-2xl font-semibold text-text-primary">SSH & GPG Keys</h1>
			<p class="mt-1 text-sm text-text-secondary">
				Manage authentication and signing keys for your account.
			</p>
		</div>
		<a
			href="/settings"
			class="text-sm text-text-secondary transition-colors hover:text-text-primary"
		>
			Back to settings
		</a>
	</div>

	{#if !auth.authenticated}
		<div class="rounded-lg border border-border bg-surface-1 p-8 text-center">
			<p class="text-text-secondary">Sign in to manage your keys.</p>
		</div>
	{:else}
		<!-- Tab switcher -->
		<div class="mb-6 flex items-center gap-1 border-b border-border">
			<button
				onclick={() => { activeTab = 'ssh'; }}
				class="border-b-2 px-4 py-2 text-sm font-medium transition-colors
					{activeTab === 'ssh'
						? 'border-accent text-text-primary'
						: 'border-transparent text-text-secondary hover:text-text-primary'}"
			>
				SSH Keys
			</button>
			<button
				onclick={() => { activeTab = 'gpg'; }}
				class="border-b-2 px-4 py-2 text-sm font-medium transition-colors
					{activeTab === 'gpg'
						? 'border-accent text-text-primary'
						: 'border-transparent text-text-secondary hover:text-text-primary'}"
			>
				GPG Keys
			</button>
			<button
				onclick={() => { activeTab = 'push'; }}
				class="border-b-2 px-4 py-2 text-sm font-medium transition-colors
					{activeTab === 'push'
						? 'border-accent text-text-primary'
						: 'border-transparent text-text-secondary hover:text-text-primary'}"
			>
				Push Tokens
			</button>
		</div>

		{#if activeTab === 'push'}
			<!-- Push Tokens tab -->
			<div class="rounded-lg border border-border bg-surface-1 p-6">
				<h2 class="mb-2 text-sm font-medium text-text-primary">Git Push Token</h2>
				<p class="mb-4 text-xs text-text-secondary">
					Generate a short-lived token to authenticate <code class="rounded bg-surface-2 px-1">git push</code> to cospan-node.
					Tokens expire after 1 hour.
				</p>

				<div class="mb-4 rounded-md border border-border bg-surface-0 p-3 text-xs text-text-secondary">
					<p class="mb-2 font-medium text-text-primary">Usage:</p>
					<ol class="list-inside list-decimal space-y-1">
						<li>Click "Generate Token" below</li>
						<li>Copy the token</li>
						<li>When git prompts for credentials:</li>
					</ol>
					<div class="mt-2 rounded bg-surface-2 px-3 py-2 font-mono text-[11px]">
						<div>Username: <span class="text-accent">{auth.did ?? 'your-did'}</span></div>
						<div>Password: <span class="text-accent">(paste the token)</span></div>
					</div>
					<div class="mt-2 font-mono text-[11px] text-text-muted">
						git remote add cospan https://node.cospan.dev/{auth.did ?? 'your-did'}/REPO<br/>
						git push cospan main
					</div>
				</div>

				{#if pushToken}
					<div class="mb-4 rounded-md border border-emerald-500/20 bg-emerald-500/5 p-3">
						<div class="mb-2 flex items-center justify-between">
							<span class="text-xs font-medium text-emerald-400">Token generated (expires in 1 hour)</span>
							<button
								onclick={copyToken}
								class="rounded bg-emerald-500/15 px-2 py-1 text-[11px] font-medium text-emerald-400 transition-colors hover:bg-emerald-500/25"
							>
								{pushTokenCopied ? 'Copied!' : 'Copy'}
							</button>
						</div>
						<code class="block break-all rounded bg-surface-2 px-3 py-2 font-mono text-[11px] text-text-primary">
							{pushToken}
						</code>
						<p class="mt-2 text-[10px] text-text-muted">
							This token will not be shown again. Generate a new one if it expires.
						</p>
					</div>
				{/if}

				{#if pushTokenError}
					<div class="mb-4 rounded-md bg-red-500/10 px-3 py-2 text-xs text-red-400">
						{pushTokenError}
					</div>
				{/if}

				<form method="POST" action="?/createPushToken" use:enhance>
					<button
						type="submit"
						class="rounded-md bg-accent px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-accent-hover"
					>
						Generate Token
					</button>
				</form>
			</div>
		{:else}
		<!-- Add key button / form -->
		{#if showForm}
			<div class="mb-6 rounded-lg border border-border bg-surface-1 p-6">
				<div class="mb-4 flex items-center justify-between">
					<h2 class="text-sm font-medium text-text-primary">Add new {activeTab.toUpperCase()} key</h2>
					<button
						onclick={() => { showForm = false; }}
						class="text-xs text-text-secondary transition-colors hover:text-text-primary"
					>
						Cancel
					</button>
				</div>
				<KeyForm onsubmit={handleAddKey} />
			</div>
		{:else}
			<div class="mb-6">
				<button
					onclick={() => { showForm = true; }}
					class="rounded-md bg-accent px-3 py-1.5 text-sm font-medium text-white transition-colors hover:bg-accent-hover"
				>
					New {activeTab.toUpperCase()} key
				</button>
			</div>
		{/if}

		<!-- Key list -->
		{#if loading}
			<div class="space-y-3">
				{#each Array(3) as _}
					<div class="h-16 animate-pulse rounded-lg bg-surface-2"></div>
				{/each}
			</div>
		{:else if filteredKeys.length === 0}
			<div class="flex flex-col items-center gap-3 py-12 text-text-secondary">
				<svg class="h-10 w-10" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
					<path stroke-linecap="round" stroke-linejoin="round" d="M15.75 5.25a3 3 0 013 3m3 0a6 6 0 01-7.029 5.912c-.563-.097-1.159.026-1.563.43L10.5 17.25H8.25v2.25H6v2.25H2.25v-2.818c0-.597.237-1.17.659-1.591l6.499-6.499c.404-.404.527-1 .43-1.563A6 6 0 1121.75 8.25z" />
				</svg>
				<p>No {activeTab.toUpperCase()} keys added yet.</p>
			</div>
		{:else}
			<div class="divide-y divide-border rounded-lg border border-border bg-surface-1">
				{#each filteredKeys as key (key.rkey)}
					<div class="flex items-start justify-between px-4 py-3">
						<div class="min-w-0 flex-1">
							<div class="flex items-center gap-2">
								<svg class="h-4 w-4 shrink-0 text-text-secondary" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
									<path stroke-linecap="round" stroke-linejoin="round" d="M15.75 5.25a3 3 0 013 3m3 0a6 6 0 01-7.029 5.912c-.563-.097-1.159.026-1.563.43L10.5 17.25H8.25v2.25H6v2.25H2.25v-2.818c0-.597.237-1.17.659-1.591l6.499-6.499c.404-.404.527-1 .43-1.563A6 6 0 1121.75 8.25z" />
								</svg>
								<span class="text-sm font-medium text-text-primary">{key.title}</span>
							</div>
							<p class="mt-1 truncate font-mono text-xs text-text-secondary">
								{truncateFingerprint(key.fingerprint)}
							</p>
							<p class="mt-0.5 text-xs text-text-secondary">
								Added {formatDate(key.createdAt)}
							</p>
						</div>
						<button
							onclick={() => handleDelete(key.rkey)}
							disabled={deletingKey === key.rkey}
							class="ml-4 shrink-0 rounded-md border border-border px-2.5 py-1 text-xs text-breaking transition-colors hover:bg-breaking/10 disabled:opacity-50"
						>
							{deletingKey === key.rkey ? 'Deleting...' : 'Delete'}
						</button>
					</div>
				{/each}
			</div>
		{/if}
		{/if}<!-- close push tab {:else} -->
	{/if}
</section>
