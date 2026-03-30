<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import Breadcrumb from '$lib/components/shared/Breadcrumb.svelte';
	import { getAuth } from '$lib/stores/auth.svelte';
	import { createWebhook, WEBHOOK_EVENTS, type WebhookEvent } from '$lib/api/webhook.js';

	let did = $derived($page.params.did);
	let repo = $derived($page.params.repo);
	let basePath = $derived(`/${did}/${repo}`);
	let auth = $derived(getAuth());

	let url = $state('');
	let secret = $state('');
	let selectedEvents = $state<WebhookEvent[]>([]);
	let submitting = $state(false);
	let error = $state('');

	let crumbs = $derived([
		{ label: did, href: `/${did}` },
		{ label: repo, href: basePath },
		{ label: 'Settings', href: `${basePath}/settings` },
		{ label: 'Webhooks', href: `${basePath}/settings/webhooks` },
		{ label: 'New' },
	]);

	function toggleEvent(event: WebhookEvent) {
		const idx = selectedEvents.indexOf(event);
		if (idx >= 0) {
			selectedEvents = [...selectedEvents.slice(0, idx), ...selectedEvents.slice(idx + 1)];
		} else {
			selectedEvents = [...selectedEvents, event];
		}
	}

	async function handleSubmit() {
		if (!url.trim() || selectedEvents.length === 0 || submitting) return;
		submitting = true;
		error = '';

		try {
			await createWebhook({
				did,
				repo,
				url: url.trim(),
				secret: secret.trim() || undefined,
				events: selectedEvents,
			});
			await goto(`${basePath}/settings/webhooks`);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to create webhook';
		} finally {
			submitting = false;
		}
	}
</script>

<svelte:head>
	<title>New Webhook · {repo} Settings · Cospan</title>
</svelte:head>

<section class="mx-auto max-w-2xl">
	<div class="mb-6">
		<Breadcrumb {crumbs} />
		<h1 class="mt-3 text-xl font-semibold text-text-primary">Add webhook</h1>
		<p class="mt-1 text-sm text-text-secondary">
			Configure a URL to receive POST requests when events occur in this repository.
		</p>
	</div>

	{#if !auth.authenticated}
		<div class="rounded-lg border border-border bg-surface-1 p-8 text-center">
			<p class="text-text-secondary">Sign in to create webhooks.</p>
		</div>
	{:else}
		<form onsubmit={handleSubmit} class="space-y-6">
			<div class="rounded-lg border border-border bg-surface-1 p-6 space-y-4">
				<div>
					<label for="webhook-url" class="mb-1 block text-xs font-medium text-text-secondary">
						Payload URL
					</label>
					<input
						id="webhook-url"
						bind:value={url}
						type="url"
						placeholder="https://example.com/webhook"
						required
						class="w-full rounded-md border border-border bg-surface-0 px-3 py-2 text-sm text-text-primary placeholder:text-text-secondary focus:border-accent focus:outline-none"
					/>
				</div>

				<div>
					<label for="webhook-secret" class="mb-1 block text-xs font-medium text-text-secondary">
						Secret
					</label>
					<input
						id="webhook-secret"
						bind:value={secret}
						type="password"
						placeholder="Optional shared secret for signature verification"
						class="w-full rounded-md border border-border bg-surface-0 px-3 py-2 text-sm text-text-primary placeholder:text-text-secondary focus:border-accent focus:outline-none"
					/>
					<p class="mt-1 text-xs text-text-secondary">
						Used to compute an HMAC signature for each delivery.
					</p>
				</div>

				<div>
					<p class="mb-2 text-xs font-medium text-text-secondary">Events</p>
					<div class="space-y-2">
						{#each WEBHOOK_EVENTS as evt (evt.value)}
							<label class="flex items-start gap-3 rounded-md px-2 py-1.5 transition-colors hover:bg-surface-2 cursor-pointer">
								<input
									type="checkbox"
									checked={selectedEvents.includes(evt.value)}
									onchange={() => toggleEvent(evt.value)}
									class="mt-0.5 rounded border-border text-accent focus:ring-accent"
								/>
								<div>
									<span class="text-sm font-medium text-text-primary">{evt.label}</span>
									<p class="text-xs text-text-secondary">{evt.description}</p>
								</div>
							</label>
						{/each}
					</div>
				</div>
			</div>

			{#if error}
				<p class="text-sm text-breaking">{error}</p>
			{/if}

			<div class="flex items-center gap-3">
				<button
					type="submit"
					disabled={submitting || !url.trim() || selectedEvents.length === 0}
					class="rounded-md bg-accent px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-accent-hover disabled:opacity-50"
				>
					{submitting ? 'Creating...' : 'Create webhook'}
				</button>
				<a
					href="{basePath}/settings/webhooks"
					class="text-sm text-text-secondary transition-colors hover:text-text-primary"
				>
					Cancel
				</a>
			</div>
		</form>
	{/if}
</section>
