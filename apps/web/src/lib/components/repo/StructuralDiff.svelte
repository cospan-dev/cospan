<script lang="ts">
	import type { DiffCommitsResponse, DiffFile, DiffLine } from '$lib/api/vcs.js';

	let { diff }: { diff: DiffCommitsResponse } = $props();

	// Per-file collapse state. Modified files start expanded, others collapsed.
	let openPaths = $state(
		new Set<string>(
			diff.files
				.filter((f) => f.status === 'modified' || f.status === 'added' || f.status === 'removed')
				.slice(0, 5)
				.map((f) => f.path)
		)
	);

	function toggle(path: string) {
		const next = new Set(openPaths);
		if (next.has(path)) next.delete(path);
		else next.add(path);
		openPaths = next;
	}

	function statusBadge(status: string): { bg: string; fg: string; label: string } {
		switch (status) {
			case 'added':
				return { bg: 'bg-compatible/15', fg: 'text-compatible', label: 'added' };
			case 'removed':
				return { bg: 'bg-breaking/15', fg: 'text-breaking', label: 'removed' };
			case 'renamed':
				return { bg: 'bg-info/15', fg: 'text-info', label: 'renamed' };
			case 'copied':
				return { bg: 'bg-info/15', fg: 'text-info', label: 'copied' };
			case 'typechange':
				return { bg: 'bg-warning/15', fg: 'text-warning', label: 'typechange' };
			default:
				return { bg: 'bg-surface-2', fg: 'text-text-secondary', label: 'modified' };
		}
	}

	function lineClass(origin: string): string {
		switch (origin) {
			case '+':
				return 'bg-compatible/10 text-compatible';
			case '-':
				return 'bg-breaking/10 text-breaking';
			default:
				return 'text-text-secondary';
		}
	}

	function linePrefix(origin: string): string {
		if (origin === '+') return '+';
		if (origin === '-') return '-';
		return ' ';
	}

	function stripNewline(s: string): string {
		return s.replace(/\n$/, '');
	}
</script>

{#if diff.files.length === 0}
	<div class="rounded-lg border border-border bg-surface-1 p-8 text-center">
		<p class="text-sm text-text-secondary">No changes between these commits.</p>
	</div>
{:else}
	<!-- Summary bar -->
	<div class="mb-4 flex flex-wrap items-center gap-4 rounded-lg border border-border bg-surface-1 px-4 py-3 text-sm">
		<span class="font-medium text-text-primary">
			{diff.fileCount} {diff.fileCount === 1 ? 'file' : 'files'} changed
		</span>
		<span class="flex items-center gap-1 text-compatible">
			<span class="font-mono">+{diff.totalAdditions}</span>
		</span>
		<span class="flex items-center gap-1 text-breaking">
			<span class="font-mono">−{diff.totalDeletions}</span>
		</span>
	</div>

	<!-- File list -->
	<div class="space-y-3">
		{#each diff.files as file (file.path)}
			{@const badge = statusBadge(file.status)}
			{@const isOpen = openPaths.has(file.path)}
			<div class="overflow-hidden rounded-lg border border-border bg-surface-1">
				<!-- File header -->
				<button
					type="button"
					class="flex w-full items-center gap-3 px-4 py-3 text-left transition-colors hover:bg-surface-2"
					onclick={() => toggle(file.path)}
				>
					<svg
						class="h-4 w-4 shrink-0 text-text-muted transition-transform {isOpen ? 'rotate-90' : ''}"
						fill="none"
						viewBox="0 0 24 24"
						stroke="currentColor"
						stroke-width="2"
					>
						<path stroke-linecap="round" stroke-linejoin="round" d="M9 5l7 7-7 7" />
					</svg>
					<span class="rounded px-2 py-0.5 text-[10px] font-medium uppercase tracking-wider {badge.bg} {badge.fg}">
						{badge.label}
					</span>
					<code class="min-w-0 flex-1 truncate font-mono text-sm text-text-primary">
						{#if file.oldPath && file.oldPath !== file.path}
							<span class="text-text-muted">{file.oldPath} → </span>
						{/if}
						{file.path}
					</code>
					{#if file.additions > 0}
						<span class="shrink-0 font-mono text-xs text-compatible">+{file.additions}</span>
					{/if}
					{#if file.deletions > 0}
						<span class="shrink-0 font-mono text-xs text-breaking">−{file.deletions}</span>
					{/if}
				</button>

				<!-- Hunks -->
				{#if isOpen}
					{#if file.binary}
						<div class="border-t border-border bg-surface-0 px-4 py-6 text-center text-xs text-text-muted">
							Binary file — no diff shown
						</div>
					{:else if file.hunks.length === 0}
						<div class="border-t border-border bg-surface-0 px-4 py-6 text-center text-xs text-text-muted">
							{file.status === 'added'
								? 'New empty file'
								: file.status === 'removed'
									? 'File deleted — no content to show'
									: 'No content changes (mode or metadata only)'}
						</div>
					{:else}
						{#each file.hunks as hunk, h (h)}
							<div class="border-t border-border">
								<div class="bg-surface-2 px-4 py-1.5 font-mono text-[11px] text-text-muted">
									{hunk.header.replace(/\n$/, '')}
								</div>
								<pre class="overflow-x-auto bg-surface-0 font-mono text-[12px] leading-5">{#each hunk.lines as line, l (l)}<span class={lineClass(line.origin)}><span class="inline-block w-10 select-none text-right pr-2 text-text-muted">{line.oldLineno ?? ''}</span><span class="inline-block w-10 select-none text-right pr-2 text-text-muted">{line.newLineno ?? ''}</span><span class="inline-block w-4 select-none">{linePrefix(line.origin)}</span>{stripNewline(line.content)}
</span>{/each}</pre>
							</div>
						{/each}
					{/if}
				{/if}
			</div>
		{/each}
	</div>
{/if}
