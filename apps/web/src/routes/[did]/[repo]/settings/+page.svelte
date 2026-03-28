<script lang="ts">
	import { goto } from '$app/navigation';
	import { getAuth } from '$lib/stores/auth.svelte';
	import Breadcrumb from '$lib/components/shared/Breadcrumb.svelte';
	import RepoTabBar from '$lib/components/repo/RepoTabBar.svelte';
	import BackLink from '$lib/components/shared/BackLink.svelte';

	let { data } = $props();

	let auth = $derived(getAuth());
	let isOwner = $derived(auth.authenticated && data.did === auth.did);
	let basePath = $derived(`/${data.did}/${data.repoName}`);

	let crumbs = $derived([
		{ label: data.did, href: `/${data.did}` },
		{ label: data.repoName, href: basePath },
		{ label: 'Settings' }
	]);

	// General settings
	let repoDescription = $state(data.repo?.description ?? '');
	let defaultBranch = $state('main');
	let savingGeneral = $state(false);
	let generalError = $state('');
	let generalSuccess = $state('');

	// Collaborators
	let collaboratorDid = $state('');
	let collaboratorRole = $state('read');
	let addingCollaborator = $state(false);
	let collaborators = $state<{ did: string; handle: string | null; role: string }[]>([]);

	// Labels
	let newLabelName = $state('');
	let newLabelColor = $state('#6366f1');
	let addingLabel = $state(false);
	let labels = $state<{ name: string; color: string }[]>([]);

	// Danger zone
	let deleteConfirmName = $state('');
	let deleting = $state(false);
	let archiving = $state(false);

	async function saveGeneral() {
		if (savingGeneral) return;
		savingGeneral = true;
		generalError = '';
		generalSuccess = '';

		try {
			const resp = await fetch('/xrpc/dev.cospan.repo.update', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					did: data.did,
					name: data.repoName,
					description: repoDescription,
					defaultBranch,
				}),
			});

			if (!resp.ok) {
				const body = await resp.json().catch(() => ({}));
				generalError = body.message ?? `Save failed (${resp.status})`;
			} else {
				generalSuccess = 'Settings saved.';
			}
		} catch (e) {
			generalError = e instanceof Error ? e.message : 'Save failed.';
		} finally {
			savingGeneral = false;
		}
	}

	async function addCollaborator() {
		if (!collaboratorDid.trim() || addingCollaborator) return;
		addingCollaborator = true;

		try {
			const resp = await fetch('/xrpc/dev.cospan.repo.collaborator.add', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					repoDid: data.did,
					repoName: data.repoName,
					collaboratorDid: collaboratorDid.trim(),
					role: collaboratorRole,
				}),
			});

			if (resp.ok) {
				collaborators = [...collaborators, { did: collaboratorDid.trim(), handle: null, role: collaboratorRole }];
				collaboratorDid = '';
			}
		} catch {
			// Add collaborator failed.
		} finally {
			addingCollaborator = false;
		}
	}

	async function removeCollaborator(did: string) {
		try {
			await fetch('/xrpc/dev.cospan.repo.collaborator.remove', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					repoDid: data.did,
					repoName: data.repoName,
					collaboratorDid: did,
				}),
			});
			collaborators = collaborators.filter((c) => c.did !== did);
		} catch {
			// Remove failed.
		}
	}

	async function addLabel() {
		if (!newLabelName.trim() || addingLabel) return;
		addingLabel = true;

		try {
			const resp = await fetch('/xrpc/dev.cospan.label.definition.create', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					repoDid: data.did,
					repoName: data.repoName,
					name: newLabelName.trim(),
					color: newLabelColor,
				}),
			});

			if (resp.ok) {
				labels = [...labels, { name: newLabelName.trim(), color: newLabelColor }];
				newLabelName = '';
				newLabelColor = '#6366f1';
			}
		} catch {
			// Add label failed.
		} finally {
			addingLabel = false;
		}
	}

	async function archiveRepo() {
		if (archiving) return;
		archiving = true;

		try {
			await fetch('/xrpc/dev.cospan.repo.archive', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ did: data.did, name: data.repoName }),
			});
		} catch {
			// Archive failed.
		} finally {
			archiving = false;
		}
	}

	async function deleteRepo() {
		if (deleteConfirmName !== data.repoName || deleting) return;
		deleting = true;

		try {
			const resp = await fetch('/xrpc/dev.cospan.repo.delete', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ did: data.did, name: data.repoName }),
			});

			if (resp.ok) {
				goto(`/${data.did}`);
			}
		} catch {
			// Delete failed.
		} finally {
			deleting = false;
		}
	}
</script>

<svelte:head>
	<title>Settings - {data.repoName} - Cospan</title>
</svelte:head>

<section>
	<Breadcrumb {crumbs} />

	<h1 class="mt-3 mb-6 text-xl font-semibold text-text-primary">Repository Settings</h1>

	<RepoTabBar {basePath} activeTab="settings" isOwner={true} />

	{#if !isOwner}
		<div class="mt-8 flex flex-col items-center gap-3 py-12 text-text-secondary">
			<p class="text-sm">You do not have permission to view these settings.</p>
			<a
				href={basePath}
				class="mt-2 rounded-md border border-border bg-surface-1 px-4 py-2 text-xs text-text-secondary transition-colors hover:border-accent hover:text-text-primary"
			>
				Back to repository
			</a>
		</div>
	{:else}
		<!-- General -->
		<div class="mt-6 rounded-lg border border-border bg-surface-1 p-5">
			<h2 class="text-sm font-semibold text-text-primary">General</h2>

			<div class="mt-4 space-y-4">
				<div>
					<label for="repo-name" class="block text-xs font-medium text-text-secondary">Name</label>
					<input
						id="repo-name"
						type="text"
						value={data.repoName}
						disabled
						class="mt-1 w-full rounded-md border border-border bg-surface-0 px-3 py-1.5 font-mono text-sm text-text-secondary opacity-60"
					/>
					<p class="mt-1 text-[11px] text-text-secondary">Repository names cannot be changed after creation.</p>
				</div>

				<div>
					<label for="repo-desc" class="block text-xs font-medium text-text-secondary">Description</label>
					<textarea
						id="repo-desc"
						bind:value={repoDescription}
						rows="3"
						class="mt-1 w-full rounded-md border border-border bg-surface-0 px-3 py-1.5 text-sm text-text-primary placeholder:text-text-secondary focus:border-accent focus:outline-none"
						placeholder="A brief description of this repository"
					></textarea>
				</div>

				<div>
					<label for="default-branch" class="block text-xs font-medium text-text-secondary">Default branch</label>
					<input
						id="default-branch"
						type="text"
						bind:value={defaultBranch}
						class="mt-1 w-full rounded-md border border-border bg-surface-0 px-3 py-1.5 font-mono text-sm text-text-primary focus:border-accent focus:outline-none"
					/>
				</div>

				<div>
					<span class="block text-xs font-medium text-text-secondary">Protocol</span>
					<span class="mt-1 inline-block rounded-full bg-surface-2 px-2 py-0.5 font-mono text-xs text-text-secondary">
						{data.repo?.protocol ?? 'unknown'}
					</span>
					<p class="mt-1 text-[11px] text-text-secondary">Protocol is determined at creation time and cannot be changed.</p>
				</div>

				{#if generalError}
					<div class="rounded-md bg-breaking/10 px-3 py-2 text-sm text-breaking">{generalError}</div>
				{/if}
				{#if generalSuccess}
					<div class="rounded-md bg-compatible/10 px-3 py-2 text-sm text-compatible">{generalSuccess}</div>
				{/if}

				<div class="flex justify-end">
					<button
						onclick={saveGeneral}
						disabled={savingGeneral}
						class="rounded-md bg-accent px-4 py-2 text-sm font-medium text-surface-0 transition-colors hover:bg-accent-hover disabled:opacity-50"
					>
						{savingGeneral ? 'Saving...' : 'Save changes'}
					</button>
				</div>
			</div>
		</div>

		<!-- Collaborators -->
		<div class="mt-6 rounded-lg border border-border bg-surface-1 p-5">
			<h2 class="text-sm font-semibold text-text-primary">Collaborators</h2>
			<p class="mt-1 text-xs text-text-secondary">Manage who has access to this repository.</p>

			{#if collaborators.length > 0}
				<ul class="mt-4 divide-y divide-border rounded-md border border-border">
					{#each collaborators as collab (collab.did)}
						<li class="flex items-center justify-between px-3 py-2">
							<div class="flex items-center gap-2">
								<span class="font-mono text-xs text-text-primary">{collab.handle ?? collab.did}</span>
								<span class="rounded bg-surface-2 px-1.5 py-0.5 text-[10px] text-text-secondary">{collab.role}</span>
							</div>
							<button
								onclick={() => removeCollaborator(collab.did)}
								class="text-xs text-breaking transition-colors hover:text-breaking/80"
							>
								Remove
							</button>
						</li>
					{/each}
				</ul>
			{/if}

			<div class="mt-4 flex items-end gap-2">
				<div class="flex-1">
					<label for="collab-did" class="block text-xs font-medium text-text-secondary">DID or handle</label>
					<input
						id="collab-did"
						type="text"
						bind:value={collaboratorDid}
						placeholder="did:plc:... or alice.bsky.social"
						class="mt-1 w-full rounded-md border border-border bg-surface-0 px-3 py-1.5 font-mono text-sm text-text-primary placeholder:text-text-secondary focus:border-accent focus:outline-none"
					/>
				</div>
				<div class="w-28">
					<label for="collab-role" class="block text-xs font-medium text-text-secondary">Role</label>
					<select
						id="collab-role"
						bind:value={collaboratorRole}
						class="mt-1 w-full rounded-md border border-border bg-surface-0 px-3 py-1.5 text-sm text-text-primary focus:border-accent focus:outline-none"
					>
						<option value="read">Read</option>
						<option value="write">Write</option>
						<option value="admin">Admin</option>
					</select>
				</div>
				<button
					onclick={addCollaborator}
					disabled={!collaboratorDid.trim() || addingCollaborator}
					class="rounded-md bg-accent px-4 py-1.5 text-sm font-medium text-surface-0 transition-colors hover:bg-accent-hover disabled:opacity-50"
				>
					{addingCollaborator ? 'Adding...' : 'Add'}
				</button>
			</div>
		</div>

		<!-- Labels -->
		<div class="mt-6 rounded-lg border border-border bg-surface-1 p-5">
			<h2 class="text-sm font-semibold text-text-primary">Labels</h2>
			<p class="mt-1 text-xs text-text-secondary">Create labels for issues and merge requests.</p>

			{#if labels.length > 0}
				<div class="mt-4 flex flex-wrap gap-2">
					{#each labels as label (label.name)}
						<span
							class="inline-flex items-center gap-1 rounded-full px-2.5 py-0.5 text-xs font-medium"
							style="background-color: {label.color}20; color: {label.color};"
						>
							<span class="h-2 w-2 rounded-full" style="background-color: {label.color};"></span>
							{label.name}
						</span>
					{/each}
				</div>
			{/if}

			<div class="mt-4 flex items-end gap-2">
				<div class="flex-1">
					<label for="label-name" class="block text-xs font-medium text-text-secondary">Label name</label>
					<input
						id="label-name"
						type="text"
						bind:value={newLabelName}
						placeholder="bug, enhancement, breaking..."
						class="mt-1 w-full rounded-md border border-border bg-surface-0 px-3 py-1.5 text-sm text-text-primary placeholder:text-text-secondary focus:border-accent focus:outline-none"
					/>
				</div>
				<div class="w-20">
					<label for="label-color" class="block text-xs font-medium text-text-secondary">Color</label>
					<div class="mt-1 flex items-center gap-1">
						<input
							id="label-color"
							type="color"
							bind:value={newLabelColor}
							class="h-8 w-8 cursor-pointer rounded border border-border bg-surface-0"
						/>
						<span class="font-mono text-[10px] text-text-secondary">{newLabelColor}</span>
					</div>
				</div>
				<button
					onclick={addLabel}
					disabled={!newLabelName.trim() || addingLabel}
					class="rounded-md bg-accent px-4 py-1.5 text-sm font-medium text-surface-0 transition-colors hover:bg-accent-hover disabled:opacity-50"
				>
					{addingLabel ? 'Creating...' : 'Create'}
				</button>
			</div>
		</div>

		<!-- Danger Zone -->
		<div class="mt-6 rounded-lg border border-breaking/30 bg-surface-1 p-5">
			<h2 class="text-sm font-semibold text-breaking">Danger Zone</h2>

			<div class="mt-4 space-y-4">
				<!-- Archive -->
				<div class="flex items-center justify-between rounded-md border border-border px-4 py-3">
					<div>
						<h3 class="text-sm font-medium text-text-primary">Archive this repository</h3>
						<p class="text-xs text-text-secondary">Mark as read-only. This can be reversed.</p>
					</div>
					<button
						onclick={archiveRepo}
						disabled={archiving}
						class="rounded-md border border-breaking/40 px-3 py-1.5 text-xs font-medium text-breaking transition-colors hover:bg-breaking/10 disabled:opacity-50"
					>
						{archiving ? 'Archiving...' : 'Archive'}
					</button>
				</div>

				<!-- Delete -->
				<div class="rounded-md border border-breaking/40 px-4 py-3">
					<div class="flex items-center justify-between">
						<div>
							<h3 class="text-sm font-medium text-text-primary">Delete this repository</h3>
							<p class="text-xs text-text-secondary">
								This action is permanent. All data, issues, and merge requests will be lost.
							</p>
						</div>
					</div>
					<div class="mt-3 flex items-end gap-2">
						<div class="flex-1">
							<label for="delete-confirm" class="block text-xs text-text-secondary">
								Type <span class="font-mono font-medium text-text-primary">{data.repoName}</span> to confirm
							</label>
							<input
								id="delete-confirm"
								type="text"
								bind:value={deleteConfirmName}
								placeholder={data.repoName}
								class="mt-1 w-full rounded-md border border-border bg-surface-0 px-3 py-1.5 font-mono text-sm text-text-primary placeholder:text-text-secondary focus:border-breaking focus:outline-none"
							/>
						</div>
						<button
							onclick={deleteRepo}
							disabled={deleteConfirmName !== data.repoName || deleting}
							class="rounded-md bg-breaking px-4 py-1.5 text-sm font-medium text-surface-0 transition-colors hover:bg-breaking/80 disabled:opacity-50 disabled:cursor-default"
						>
							{deleting ? 'Deleting...' : 'Delete repository'}
						</button>
					</div>
				</div>
			</div>
		</div>
	{/if}

	<BackLink href={basePath} />
</section>
