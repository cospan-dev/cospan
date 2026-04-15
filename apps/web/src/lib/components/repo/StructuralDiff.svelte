<script lang="ts">
	import type { DiffCommitsResponse, DiffFile } from '$lib/api/vcs.js';

	let { diff }: { diff: DiffCommitsResponse } = $props();

	let openPaths = $state(
		new Set<string>(
			diff.files
				.filter((f) => !f.binary)
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
		const map: Record<string, { bg: string; fg: string; label: string }> = {
			added:      { bg: 'bg-emerald-500/15', fg: 'text-emerald-400', label: 'added' },
			removed:    { bg: 'bg-red-500/15',     fg: 'text-red-400',     label: 'removed' },
			modified:   { bg: 'bg-amber-500/15',   fg: 'text-amber-400',   label: 'modified' },
			renamed:    { bg: 'bg-blue-500/15',     fg: 'text-blue-400',    label: 'renamed' },
			copied:     { bg: 'bg-blue-500/15',     fg: 'text-blue-400',    label: 'copied' },
			typechange: { bg: 'bg-purple-500/15',   fg: 'text-purple-400',  label: 'typechange' },
		};
		return map[status] ?? map.modified;
	}

	function lineClass(origin: string): string {
		if (origin === '+') return 'bg-emerald-500/10 text-emerald-300';
		if (origin === '-') return 'bg-red-500/10 text-red-300';
		return 'text-text-secondary';
	}

	function linePrefix(origin: string): string {
		if (origin === '+') return '+';
		if (origin === '-') return '-';
		return ' ';
	}

	function stripNewline(s: string): string {
		return s.replace(/\n$/, '');
	}

	// TODO: replace with generated types from dev.panproto.node.diffCommits lexicon
	interface ScopeChange {
		scope_id: string;
		scope_name: string;
		kind: 'Added' | 'Removed' | 'SignatureChanged' | 'BodyModified';
		summary: string;
		anonymous_added: number;
		anonymous_removed: number;
		start_line: number | null;
		end_line: number | null;
	}

	interface NamedElement {
		id: string;
		name: string;
		kind: string;
		status: 'Unchanged' | 'BodyModified' | 'SignatureChanged' | 'Added' | 'Removed';
		start_line: number | null;
	}

	interface StructuralDiffData {
		protocol: string;
		compatible: boolean;
		verdict: string;
		breakingCount: number;
		nonBreakingCount: number;
		breakingChanges: { kind: string; label: string }[];
		nonBreakingChanges: { kind: string; label: string }[];
		scopeChanges?: ScopeChange[];
		namedElements?: NamedElement[];
	}

	function hasStructuralDiff(f: DiffFile): boolean {
		return !!(f as any).structuralDiff;
	}
	function getStructuralDiff(f: DiffFile): StructuralDiffData | null {
		return (f as any).structuralDiff ?? null;
	}

	// Match diff hunks to a scope by line range overlap.
	function getHunksForScope(hunks: DiffFile['hunks'], scope: ScopeChange): DiffFile['hunks'][0][] {
		if (scope.start_line == null || scope.end_line == null) {
			// Fallback: match by scope name in hunk header or line content
			return hunks.filter(h =>
				h.header.includes(scope.scope_name) ||
				h.lines.some(l => l.content.includes(scope.scope_name))
			);
		}
		return hunks.filter(h => {
			const hunkEnd = h.newStart + h.newLines;
			return h.newStart <= scope.end_line! && hunkEnd >= scope.start_line!;
		});
	}

	// Scope change kind indicator
	function scopeKindIndicator(kind: string): { symbol: string; color: string } {
		switch (kind) {
			case 'Added': return { symbol: '+', color: 'text-emerald-400' };
			case 'Removed': return { symbol: '−', color: 'text-red-400' };
			case 'SignatureChanged': return { symbol: '~', color: 'text-red-300' };
			case 'BodyModified': return { symbol: '~', color: 'text-amber-400' };
			default: return { symbol: '?', color: 'text-text-muted' };
		}
	}

	function elementStatusColor(status: string): string {
		switch (status) {
			case 'Added': return 'text-emerald-400';
			case 'Removed': return 'text-red-400';
			case 'SignatureChanged': return 'text-red-300';
			case 'BodyModified': return 'text-amber-400';
			default: return 'text-text-muted';
		}
	}

	// Map tree-sitter tags.scm canonical categories to visual symbols.
	// Since panproto v0.31.0 the walker uses tags.scm queries, so kind
	// is one of: "function", "method", "class", "module", "interface",
	// "type", "macro", or "Other:<custom>" for grammar-specific extensions.
	function scopeSymbol(kind: string): { symbol: string; label: string } {
		switch (kind.toLowerCase()) {
			case 'function': return { symbol: 'f', label: 'function' };
			case 'method':   return { symbol: 'm', label: 'method' };
			case 'class':    return { symbol: 'C', label: 'class' };
			case 'interface':return { symbol: 'I', label: 'interface' };
			case 'module':   return { symbol: 'M', label: 'module' };
			case 'type':     return { symbol: 'T', label: 'type' };
			case 'macro':    return { symbol: '#', label: 'macro' };
			default: {
				if (kind.toLowerCase().startsWith('other:')) {
					return { symbol: '.', label: kind.slice(6) };
				}
				return { symbol: '.', label: kind };
			}
		}
	}

	function elementStatusLabel(status: string): string {
		switch (status) {
			case 'Added': return 'added';
			case 'Removed': return 'removed';
			case 'SignatureChanged': return 'sig changed';
			case 'BodyModified': return 'modified';
			default: return '';
		}
	}
</script>

{#if diff.files.length === 0}
	<div class="rounded-lg border border-border bg-surface-1 p-8 text-center">
		<p class="text-sm text-text-secondary">No changes between these commits.</p>
	</div>
{:else}
	{@const noStructural = diff.files.every(f => !f.binary && !getStructuralDiff(f))}
	{#if noStructural}
		<div class="mb-4 rounded-lg border border-blue-500/30 bg-blue-500/5 p-4">
			<div class="flex items-start gap-3">
				<svg class="h-5 w-5 shrink-0 mt-0.5 text-blue-400" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
					<path stroke-linecap="round" stroke-linejoin="round" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
				</svg>
				<div class="flex-1">
					<span class="text-sm font-medium text-blue-400">Structural diff unavailable</span>
					<p class="mt-1 text-xs text-text-secondary">
						These commits were pushed via plain <code class="font-mono">git push</code>, so no pre-parsed
						schemas are available. Install <code class="font-mono">git-remote-cospan</code> and re-push to
						see scope-level changes, breaking change detection, and semantic diffs.
					</p>
				</div>
			</div>
		</div>
	{/if}
	<!-- Summary bar -->
	<div class="mb-4 flex flex-wrap items-center gap-4 rounded-lg border border-border bg-surface-1 px-4 py-3 text-sm">
		<span class="font-medium text-text-primary">
			{diff.fileCount} {diff.fileCount === 1 ? 'file' : 'files'} changed
		</span>
		<span class="flex items-center gap-1 text-emerald-400">
			<span class="font-mono">+{diff.totalAdditions}</span>
		</span>
		<span class="flex items-center gap-1 text-red-400">
			<span class="font-mono">-{diff.totalDeletions}</span>
		</span>
	</div>

	<!-- File list -->
	<div class="space-y-3">
		{#each diff.files as file (file.path)}
			{@const badge = statusBadge(file.status)}
			{@const isOpen = openPaths.has(file.path)}
			{@const sd = getStructuralDiff(file)}
			{@const isBinary = file.binary}
			<div class="overflow-hidden rounded-lg border border-border bg-surface-1">
				{#if isBinary}
					<div class="flex items-center gap-3 px-4 py-3">
						<span class="rounded px-2 py-0.5 text-[10px] font-medium uppercase tracking-wider {badge.bg} {badge.fg}">
							{badge.label}
						</span>
						<code class="min-w-0 flex-1 truncate font-mono text-sm text-text-primary">
							{file.path}
						</code>
						<span class="text-xs text-text-muted italic">binary</span>
					</div>
				{:else}
				<!-- Source file header -->
				<button
					type="button"
					class="flex w-full items-center gap-3 px-4 py-3 text-left transition-colors hover:bg-surface-2"
					onclick={() => toggle(file.path)}
				>
					<svg
						class="h-4 w-4 shrink-0 text-text-muted transition-transform {isOpen ? 'rotate-90' : ''}"
						fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"
					>
						<path stroke-linecap="round" stroke-linejoin="round" d="M9 5l7 7-7 7" />
					</svg>
					<span class="rounded px-2 py-0.5 text-[10px] font-medium uppercase tracking-wider {badge.bg} {badge.fg}">
						{badge.label}
					</span>
					<code class="min-w-0 flex-1 truncate font-mono text-sm text-text-primary">
						{#if file.oldPath && file.oldPath !== file.path}
							<span class="text-text-muted">{file.oldPath} -> </span>
						{/if}
						{file.path}
					</code>
					{#if sd}
						<span class="shrink-0 rounded px-2 py-0.5 text-[10px] font-bold uppercase tracking-wider {sd.compatible ? 'bg-emerald-500/15 text-emerald-400' : 'bg-red-500/15 text-red-400'}">
							{sd.verdict}
						</span>
						<span class="shrink-0 text-[10px] text-text-muted">{sd.protocol}</span>
					{/if}
					{#if file.additions > 0}
						<span class="shrink-0 font-mono text-xs text-emerald-400">+{file.additions}</span>
					{/if}
					{#if file.deletions > 0}
						<span class="shrink-0 font-mono text-xs text-red-400">-{file.deletions}</span>
					{/if}
				</button>

				{#if isOpen}
					{#if sd}
						<div class="border-t border-border bg-surface-0 p-4">

							<!-- Section 1: Scope changes (primary view from panproto) -->
							{#if sd.scopeChanges && sd.scopeChanges.length > 0}
								<div class="mb-4 rounded-md border border-border">
									<div class="flex items-center justify-between border-b border-border px-3 py-2">
										<span class="text-xs font-semibold uppercase tracking-wider text-text-muted">
											Changed program elements
										</span>
										<span class="text-[10px] text-text-muted">{sd.protocol}</span>
									</div>
									{#each sd.scopeChanges as scope (scope.scope_id)}
										{@const indicator = scopeKindIndicator(scope.kind)}
										{@const hunks = getHunksForScope(file.hunks, scope)}
										<details class="border-b border-border/40 last:border-0
											{scope.kind === 'Added' ? 'bg-emerald-500/3'
											 : scope.kind === 'Removed' ? 'bg-red-500/3'
											 : 'bg-surface-0'}">
											<summary class="flex cursor-pointer items-center gap-2 px-3 py-2 text-sm hover:bg-surface-2 transition-colors">
												<span class="{indicator.color} font-mono text-xs w-4 text-center">{indicator.symbol}</span>
												<span class="font-mono font-medium text-text-primary">{scope.scope_name}</span>
												<span class="text-xs text-text-muted flex-1">{scope.summary}</span>
												{#if scope.anonymous_added > 0 || scope.anonymous_removed > 0}
													<span class="text-[11px] font-mono text-text-muted">
														{#if scope.anonymous_added > 0}<span class="text-emerald-400">+{scope.anonymous_added}</span>{/if}
														{#if scope.anonymous_removed > 0}<span class="text-red-400"> -{scope.anonymous_removed}</span>{/if}
													</span>
												{/if}
												{#if scope.start_line}
													<span class="text-[10px] text-text-muted font-mono">:{scope.start_line}</span>
												{/if}
											</summary>
											<div class="border-t border-border/30">
												{#if hunks.length > 0}
													{#each hunks as hunk, h (h)}
														<div class="border-t border-border/20 first:border-t-0">
															<div class="bg-surface-2/30 px-3 py-0.5 font-mono text-[10px] text-text-muted">
																{hunk.header.replace(/\n$/, '')}
															</div>
															<pre class="overflow-x-auto bg-surface-0 font-mono text-[11px] leading-[18px]">{#each hunk.lines as line, l (l)}<span class={lineClass(line.origin)}><span class="inline-block w-7 select-none text-right pr-1 text-[10px] text-text-muted/50">{line.oldLineno ?? ''}</span><span class="inline-block w-7 select-none text-right pr-1 text-[10px] text-text-muted/50">{line.newLineno ?? ''}</span><span class="inline-block w-3 select-none text-[10px]">{linePrefix(line.origin)}</span>{stripNewline(line.content)}
</span>{/each}</pre>
														</div>
													{/each}
												{:else}
													<p class="px-3 py-2 text-[11px] text-text-muted italic">No matching code hunks</p>
												{/if}
											</div>
										</details>
									{/each}
								</div>
							{/if}

							<!-- Section 2: Full structure map (collapsed) -->
							{#if sd.namedElements && sd.namedElements.filter(e => e.status !== 'Unchanged').length > 0}
								{@const changed = sd.namedElements.filter(e => e.status !== 'Unchanged')}
								{@const unchanged = sd.namedElements.filter(e => e.status === 'Unchanged')}
								<details class="mb-4 rounded-md border border-border">
									<summary class="cursor-pointer px-3 py-2 text-xs font-semibold uppercase tracking-wider text-text-muted hover:bg-surface-2">
										All program elements ({sd.namedElements.length})
									</summary>
									<div class="divide-y divide-border/30">
										{#each changed as el (el.id)}
											{@const sym = scopeSymbol(el.kind)}
											<div class="flex items-center gap-2 px-3 py-1.5 text-sm">
												<span class="w-20 shrink-0 text-[10px] font-mono {elementStatusColor(el.status)}">
													{elementStatusLabel(el.status)}
												</span>
												<span class="w-4 text-center font-mono text-[11px] text-text-muted" title={sym.label}>
													{sym.symbol}
												</span>
												<code class="font-mono text-sm text-text-primary">{el.name}</code>
												<span class="text-[10px] text-text-muted">{sym.label}</span>
												{#if el.start_line}
													<span class="ml-auto text-[10px] text-text-muted font-mono">:{el.start_line}</span>
												{/if}
											</div>
										{/each}
										{#if unchanged.length > 0}
											<details class="border-t border-border/30">
												<summary class="cursor-pointer px-3 py-1.5 text-[11px] text-text-muted hover:text-text-secondary">
													{unchanged.length} unchanged elements
												</summary>
												{#each unchanged as el (el.id)}
													{@const sym = scopeSymbol(el.kind)}
													<div class="flex items-center gap-2 px-3 py-1 text-sm text-text-muted">
														<span class="w-20 shrink-0 text-[10px] font-mono"></span>
														<span class="w-4 text-center font-mono text-[11px]" title={sym.label}>
															{sym.symbol}
														</span>
														<code class="font-mono text-sm">{el.name}</code>
														<span class="text-[10px]">{sym.label}</span>
														{#if el.start_line}
															<span class="ml-auto text-[10px] font-mono">:{el.start_line}</span>
														{/if}
													</div>
												{/each}
											</details>
										{/if}
									</div>
								</details>
							{/if}

							<!-- Section 3: Breaking/compatible classified changes -->
							{#if sd.breakingCount > 0 || sd.nonBreakingCount > 0}
								<!-- Verdict banner -->
								<div class="mb-4 flex items-center gap-3 rounded-md px-3 py-2 {sd.compatible ? 'bg-emerald-500/10' : 'bg-red-500/10'}">
									<span class="text-lg">{sd.compatible ? '✓' : '⚠'}</span>
									<div>
										{#if !sd.compatible}
											<span class="text-sm font-semibold text-red-400">Breaking changes detected</span>
											<span class="ml-2 text-xs text-text-muted">
												{sd.breakingCount} breaking{#if sd.nonBreakingCount > 0} | {sd.nonBreakingCount} safe{/if}
											</span>
										{:else}
											<span class="text-sm font-semibold text-emerald-400">No breaking changes</span>
											<span class="ml-2 text-xs text-text-muted">
												{sd.nonBreakingCount} safe {sd.nonBreakingCount === 1 ? 'change' : 'changes'}
											</span>
										{/if}
									</div>
								</div>

								{#if sd.breakingChanges.length > 0}
									<details open class="mb-3 rounded-md border border-red-500/20">
										<summary class="cursor-pointer px-3 py-2 text-xs font-semibold uppercase tracking-wider text-red-400 hover:bg-red-500/5">
											Breaking changes ({sd.breakingCount})
										</summary>
										<div class="space-y-0.5 px-2 pb-2">
											{#each sd.breakingChanges as bc (bc.label)}
												<div class="flex items-start gap-2 rounded-md bg-red-500/5 px-3 py-1.5 text-sm">
													<span class="shrink-0 text-red-400 mt-0.5">⚠</span>
													<span class="text-text-primary flex-1">{bc.label}</span>
													<span class="shrink-0 rounded bg-surface-2 px-1.5 py-0.5 text-[10px] text-text-muted">{bc.kind}</span>
												</div>
											{/each}
										</div>
									</details>
								{/if}

								{#if sd.nonBreakingChanges.length > 0}
									<details class="mb-3 rounded-md border border-emerald-500/20">
										<summary class="cursor-pointer px-3 py-2 text-xs font-semibold uppercase tracking-wider text-emerald-400 hover:bg-emerald-500/5">
											Safe changes ({sd.nonBreakingCount})
										</summary>
										<div class="space-y-0.5 px-2 pb-2">
											{#each sd.nonBreakingChanges as nb (nb.label)}
												<div class="flex items-start gap-2 rounded-md bg-emerald-500/5 px-3 py-1.5 text-sm">
													<span class="shrink-0 text-emerald-400 mt-0.5">✓</span>
													<span class="text-text-primary flex-1">{nb.label}</span>
													<span class="shrink-0 rounded bg-surface-2 px-1.5 py-0.5 text-[10px] text-text-muted">{nb.kind}</span>
												</div>
											{/each}
										</div>
									</details>
								{/if}
							{/if}

							<!-- Section 4: Raw textual diff (collapsed) -->
							{#if file.hunks.length > 0}
								<details class="rounded-md border border-border">
									<summary class="cursor-pointer px-3 py-2 text-xs font-medium text-text-muted hover:text-text-secondary transition-colors">
										Raw textual diff ({file.additions} additions, {file.deletions} deletions)
									</summary>
									{#each file.hunks as hunk, h (h)}
										<div class="border-t border-border">
											<div class="bg-surface-2 px-3 py-1 font-mono text-[10px] text-text-muted">
												{hunk.header.replace(/\n$/, '')}
											</div>
											<pre class="overflow-x-auto bg-surface-0 font-mono text-[11px] leading-5">{#each hunk.lines as line, l (l)}<span class={lineClass(line.origin)}><span class="inline-block w-8 select-none text-right pr-1 text-text-muted">{line.oldLineno ?? ''}</span><span class="inline-block w-8 select-none text-right pr-1 text-text-muted">{line.newLineno ?? ''}</span><span class="inline-block w-3 select-none">{linePrefix(line.origin)}</span>{stripNewline(line.content)}
</span>{/each}</pre>
										</div>
									{/each}
								</details>
							{/if}
						</div>
					{:else}
						<!-- No structural diff: show raw textual diff as primary -->
						{#if file.hunks.length === 0}
							<div class="border-t border-border bg-surface-0 px-4 py-6 text-center text-xs text-text-muted">
								No content changes
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
				{/if}
				{/if}<!-- close binary check -->
			</div>
		{/each}
	</div>
{/if}
