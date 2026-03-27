<script lang="ts">
	import type { KeyType } from '$lib/api/keys.js';

	let {
		onsubmit,
	}: {
		onsubmit: (data: { type: KeyType; title: string; publicKey: string }) => Promise<void>;
	} = $props();

	let type: KeyType = $state('ssh');
	let title = $state('');
	let publicKey = $state('');
	let submitting = $state(false);
	let error = $state('');

	async function handleSubmit() {
		if (!title.trim() || !publicKey.trim() || submitting) return;
		submitting = true;
		error = '';

		try {
			await onsubmit({ type, title: title.trim(), publicKey: publicKey.trim() });
			title = '';
			publicKey = '';
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to add key';
		} finally {
			submitting = false;
		}
	}
</script>

<form onsubmit={handleSubmit} class="space-y-4">
	<div>
		<label for="key-type" class="mb-1 block text-xs font-medium text-text-secondary">
			Key type
		</label>
		<div class="flex gap-2">
			<button
				type="button"
				onclick={() => { type = 'ssh'; }}
				class="rounded-md border px-3 py-1.5 text-xs font-medium transition-colors
					{type === 'ssh'
						? 'border-accent bg-accent/10 text-accent'
						: 'border-border text-text-secondary hover:border-accent/30'}"
			>
				SSH
			</button>
			<button
				type="button"
				onclick={() => { type = 'gpg'; }}
				class="rounded-md border px-3 py-1.5 text-xs font-medium transition-colors
					{type === 'gpg'
						? 'border-accent bg-accent/10 text-accent'
						: 'border-border text-text-secondary hover:border-accent/30'}"
			>
				GPG
			</button>
		</div>
	</div>

	<div>
		<label for="key-title" class="mb-1 block text-xs font-medium text-text-secondary">
			Title
		</label>
		<input
			id="key-title"
			bind:value={title}
			type="text"
			placeholder={type === 'ssh' ? 'e.g. Work laptop' : 'e.g. Signing key'}
			class="w-full rounded-md border border-border bg-surface-0 px-3 py-2 text-sm text-text-primary placeholder:text-text-secondary focus:border-accent focus:outline-none"
		/>
	</div>

	<div>
		<label for="key-content" class="mb-1 block text-xs font-medium text-text-secondary">
			Public key
		</label>
		<textarea
			id="key-content"
			bind:value={publicKey}
			rows="6"
			placeholder={type === 'ssh'
				? 'ssh-ed25519 AAAA... user@host'
				: '-----BEGIN PGP PUBLIC KEY BLOCK-----'}
			class="w-full rounded-md border border-border bg-surface-0 px-3 py-2 font-mono text-xs text-text-primary placeholder:text-text-secondary focus:border-accent focus:outline-none resize-none"
		></textarea>
	</div>

	{#if error}
		<p class="text-sm text-breaking">{error}</p>
	{/if}

	<button
		type="submit"
		disabled={submitting || !title.trim() || !publicKey.trim()}
		class="rounded-md bg-accent px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-accent-hover disabled:opacity-50"
	>
		{submitting ? 'Adding...' : `Add ${type.toUpperCase()} key`}
	</button>
</form>
