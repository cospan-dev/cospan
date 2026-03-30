<script lang="ts">
	import { onMount } from 'svelte';
	import { getAuth } from '$lib/stores/auth.svelte';

	let auth = $derived(getAuth());

	let displayName = $state('');
	let description = $state('');
	let blueskyHandle = $state('');
	let saving = $state(false);
	let saved = $state(false);
	let error = $state('');
	let loaded = $state(false);

	onMount(async () => {
		if (!auth.authenticated || !auth.did) return;

		// Pre-fill from Bluesky profile
		try {
			const resp = await fetch(
				`https://public.api.bsky.app/xrpc/app.bsky.actor.getProfile?actor=${encodeURIComponent(auth.did)}`
			);
			if (resp.ok) {
				const data = await resp.json();
				displayName = data.displayName ?? '';
				description = data.description ?? '';
				blueskyHandle = data.handle ?? '';
			}
		} catch {}

		// Override with Cospan profile if it exists
		try {
			const resp = await fetch(
				`/xrpc/dev.cospan.actor.getProfile?did=${encodeURIComponent(auth.did)}`
			);
			if (resp.ok) {
				const data = await resp.json();
				if (data.displayName) displayName = data.displayName;
				if (data.description) description = data.description;
				if (data.bluesky) blueskyHandle = data.bluesky;
			}
		} catch {}

		loaded = true;
	});

	async function handleSave() {
		if (!auth.did || saving) return;
		saving = true;
		saved = false;
		error = '';

		try {
			// Create/update the dev.cospan.actor.profile record via the appview
			const resp = await fetch('/xrpc/dev.cospan.actor.profile.put', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					did: auth.did,
					displayName,
					description,
					bluesky: blueskyHandle,
				}),
			});

			if (resp.ok) {
				saved = true;
				setTimeout(() => { saved = false; }, 3000);
			} else {
				const body = await resp.json().catch(() => ({}));
				error = body.message ?? 'Failed to save profile';
			}
		} catch (e) {
			error = 'Network error, could not save profile';
		} finally {
			saving = false;
		}
	}
</script>

<svelte:head>
	<title>Settings - Cospan</title>
</svelte:head>

<section class="mx-auto max-w-2xl">
	<h1 class="mb-6 text-2xl font-semibold text-text-primary">Settings</h1>

	{#if !auth.authenticated}
		<div class="rounded-lg border border-border bg-surface-1 p-8 text-center">
			<p class="text-text-secondary">Sign in to manage your profile settings.</p>
		</div>
	{:else}
		<div class="space-y-6">
			<!-- Profile section -->
			<div class="rounded-lg border border-border bg-surface-1 p-6">
				<h2 class="mb-4 text-lg font-medium text-text-primary">Profile</h2>

				<div class="space-y-4">
					<div>
						<label for="displayName" class="mb-1 block text-xs font-medium text-text-secondary">
							Display name
						</label>
						<input
							id="displayName"
							bind:value={displayName}
							type="text"
							placeholder="Your name"
							class="w-full rounded-md border border-border bg-surface-0 px-3 py-2 text-sm text-text-primary placeholder:text-text-secondary focus:border-accent focus:outline-none"
						/>
					</div>

					<div>
						<label for="description" class="mb-1 block text-xs font-medium text-text-secondary">
							Bio
						</label>
						<textarea
							id="description"
							bind:value={description}
							rows="3"
							placeholder="Tell us about yourself"
							class="w-full rounded-md border border-border bg-surface-0 px-3 py-2 text-sm text-text-primary placeholder:text-text-secondary focus:border-accent focus:outline-none resize-none"
						></textarea>
					</div>

					<div>
						<label for="bluesky" class="mb-1 block text-xs font-medium text-text-secondary">
							Bluesky handle
						</label>
						<input
							id="bluesky"
							bind:value={blueskyHandle}
							type="text"
							placeholder="you.bsky.social"
							class="w-full rounded-md border border-border bg-surface-0 px-3 py-2 text-sm text-text-primary placeholder:text-text-secondary focus:border-accent focus:outline-none"
						/>
						<p class="mt-1 text-xs text-text-secondary">Links your Cospan identity to your Bluesky account.</p>
					</div>
				</div>

				<div class="mt-6 flex items-center gap-3">
					<button
						onclick={handleSave}
						disabled={saving}
						class="rounded-md bg-accent px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-accent-hover disabled:opacity-50"
					>
						{saving ? 'Saving...' : 'Save profile'}
					</button>

					{#if saved}
						<span class="text-sm text-compatible">Saved!</span>
					{/if}

					{#if error}
						<span class="text-sm text-breaking">{error}</span>
					{/if}
				</div>
			</div>

			<!-- SSH & GPG Keys -->
			<div class="rounded-lg border border-border bg-surface-1 p-6">
				<div class="flex items-center justify-between">
					<div>
						<h2 class="text-lg font-medium text-text-primary">SSH & GPG Keys</h2>
						<p class="mt-1 text-sm text-text-secondary">Manage authentication and signing keys.</p>
					</div>
					<a
						href="/settings/keys"
						class="rounded-md border border-border px-3 py-1.5 text-sm text-text-secondary transition-colors hover:border-accent hover:text-text-primary"
					>
						Manage keys
					</a>
				</div>
			</div>

			<!-- Account info -->
			<div class="rounded-lg border border-border bg-surface-1 p-6">
				<h2 class="mb-4 text-lg font-medium text-text-primary">Account</h2>
				<div class="space-y-2 text-sm">
					<div class="flex justify-between">
						<span class="text-text-secondary">DID</span>
						<span class="font-mono text-xs text-text-primary">{auth.did}</span>
					</div>
					<div class="flex justify-between">
						<span class="text-text-secondary">Handle</span>
						<span class="text-text-primary">{auth.handle}</span>
					</div>
				</div>
			</div>
		</div>
	{/if}
</section>
