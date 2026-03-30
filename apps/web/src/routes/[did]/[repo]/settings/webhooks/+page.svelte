<script lang="ts">
	import { getContext } from 'svelte';
	import { timeAgo } from '$lib/utils/time.js';
	import { getAuth } from '$lib/stores/auth.svelte';
	import { deleteWebhook } from '$lib/api/webhook.js';

	let { data } = $props();

	let auth = $derived(getAuth());
	let basePath = $derived(`/${data.did}/${data.repo}`);
	let deleting = $state<string | null>(null);

	const repoLayout = getContext<any>('repoLayout');
	$effect(() => {
		repoLayout?.setExtraCrumbs([
			{ label: 'Settings', href: `${basePath}/settings` },
			{ label: 'Webhooks' }
		]);
		return () => repoLayout?.setExtraCrumbs([]);
	});

	async function handleDelete(rkey: string) {
		if (!auth.authenticated || !auth.did || deleting) return;
		deleting = rkey;

		try {
			await deleteWebhook({ did: data.did, repo: data.repo, rkey });
			data.webhooks.items = data.webhooks.items.filter((w) => w.rkey !== rkey);
		} catch (e) {
			console.error('Failed to delete webhook:', e);
		} finally {
			deleting = null;
		}
	}
</script>

<svelte:head>
	<title>Webhooks · {data.repo} Settings · Cospan</title>
</svelte:head>

<div class="mb-6">
	<div class="mt-3 flex items-center justify-between">
		<h1 class="text-xl font-semibold text-text-primary">Webhooks</h1>
		<a
			href="{basePath}/settings/webhooks/new"
			class="rounded-md bg-accent px-3 py-1.5 text-sm font-medium text-white transition-colors hover:bg-accent-hover"
		>
			Add webhook
		</a>
	</div>
	<p class="mt-1 text-sm text-text-secondary">
		Webhooks notify external services when events occur in this repository.
	</p>
</div>

{#if data.webhooks.items.length === 0}
	<div class="flex flex-col items-center gap-4 py-12 text-text-secondary">
		<svg class="h-12 w-12" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
			<path stroke-linecap="round" stroke-linejoin="round" d="M13.19 8.688a4.5 4.5 0 011.242 7.244l-4.5 4.5a4.5 4.5 0 01-6.364-6.364l1.757-1.757m9.86-3.03l4.5-4.5a4.5 4.5 0 00-6.364-6.364l-4.5 4.5a4.5 4.5 0 001.242 7.244" />
		</svg>
		<p>No webhooks configured.</p>
		<a
			href="{basePath}/settings/webhooks/new"
			class="rounded-md bg-accent px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-accent-hover"
		>
			Add your first webhook
		</a>
	</div>
{:else}
	<div class="divide-y divide-border rounded-lg border border-border bg-surface-1">
		{#each data.webhooks.items as webhook (webhook.rkey)}
			<div class="flex items-center justify-between px-4 py-3">
				<div class="min-w-0 flex-1">
					<div class="flex items-center gap-2">
						<span class="font-mono text-sm text-text-primary truncate">{webhook.url}</span>
						{#if webhook.active}
							<span class="rounded-full bg-compatible/15 px-1.5 py-0.5 text-xs text-compatible">active</span>
						{:else}
							<span class="rounded-full bg-breaking/15 px-1.5 py-0.5 text-xs text-breaking">inactive</span>
						{/if}
					</div>
					<div class="mt-1 flex flex-wrap items-center gap-2 text-xs text-text-secondary">
						<span>Events: {webhook.events.join(', ')}</span>
						{#if webhook.lastDeliveryAt}
							<span>Last delivery: {timeAgo(webhook.lastDeliveryAt)}</span>
							{#if webhook.lastDeliveryStatus}
								<span class={webhook.lastDeliveryStatus >= 200 && webhook.lastDeliveryStatus < 300 ? 'text-compatible' : 'text-breaking'}>
									HTTP {webhook.lastDeliveryStatus}
								</span>
							{/if}
						{:else}
							<span>Never delivered</span>
						{/if}
					</div>
				</div>
				<button
					onclick={() => handleDelete(webhook.rkey)}
					disabled={deleting === webhook.rkey}
					class="ml-4 rounded-md border border-border px-2.5 py-1 text-xs text-breaking transition-colors hover:bg-breaking/10 disabled:opacity-50"
				>
					{deleting === webhook.rkey ? 'Deleting...' : 'Delete'}
				</button>
			</div>
		{/each}
	</div>
{/if}
