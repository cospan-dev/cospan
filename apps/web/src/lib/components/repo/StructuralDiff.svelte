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

	type FileStatus = 'added' | 'removed' | 'modified' | 'renamed' | 'copied' | 'typechange';
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

	// Structural diff types
	interface StructuralDiffData {
		protocol: string;
		compatible: boolean;
		verdict: string;
		breakingCount: number;
		nonBreakingCount: number;
		oldVertexCount: number;
		newVertexCount: number;
		oldEdgeCount: number;
		newEdgeCount: number;
		addedVertices: string[];
		removedVertices: string[];
		kindChanges: { vertexId: string; oldKind: string; newKind: string }[];
		addedEdges: { src: string; tgt: string; kind: string; name: string | null }[];
		removedEdges: { src: string; tgt: string; kind: string; name: string | null }[];
		breakingChanges: { kind: string; label: string }[];
		nonBreakingChanges: { kind: string; label: string }[];
	}

	function hasStructuralDiff(f: DiffFile): boolean {
		return !!(f as any).structuralDiff;
	}
	function getStructuralDiff(f: DiffFile): StructuralDiffData | null {
		return (f as any).structuralDiff ?? null;
	}

	// ── Vertex ID parsing ──────────────────────────────────────────
	// IDs look like "src/repo.ts::Repo::$2::$34" or "dev.cospan.repo:body.name"
	// Extract the nearest NAMED ancestor (non-$N component) for grouping.

	interface VertexGroup {
		scope: string;       // "Repo", "createRepo", "forkRepo", etc.
		scopeKind: string;   // icon for the scope
		items: string[];     // leaf vertex IDs in this group
	}

	function groupVertices(ids: string[], filePath: string): VertexGroup[] {
		const groups = new Map<string, string[]>();
		for (const id of ids) {
			const scope = extractScope(id, filePath);
			if (!groups.has(scope)) groups.set(scope, []);
			groups.get(scope)!.push(id);
		}
		// Merge anonymous scopes ($N) into their nearest named parent
		// or into "(other)" if no parent is found
		const named = new Map<string, string[]>();
		for (const [scope, items] of groups) {
			if (scope.startsWith('$') || scope === '') {
				const key = '(other)';
				if (!named.has(key)) named.set(key, []);
				named.get(key)!.push(...items);
			} else {
				if (!named.has(scope)) named.set(scope, []);
				named.get(scope)!.push(...items);
			}
		}
		return Array.from(named.entries())
			.filter(([_, items]) => items.length > 0)
			.map(([scope, items]) => ({
				scope,
				scopeKind: guessScopeKind(scope),
				items,
			}))
			.sort((a, b) => b.items.length - a.items.length);
	}

	function extractScope(id: string, filePath: string): string {
		// Strip the file path prefix
		let path = id;
		if (path.startsWith(filePath + '::')) {
			path = path.slice(filePath.length + 2);
		}
		// For schema-level IDs like "dev.cospan.repo:body.name"
		if (path.includes(':')) {
			const parts = path.split(':');
			// Find the first meaningful named segment
			for (const p of parts) {
				const name = p.split('.').find(s => s && !s.startsWith('$'));
				if (name && name !== 'body') return name;
			}
			return parts[0] || path;
		}
		// For tree-sitter IDs like "Repo::$2::$34"
		const segments = path.split('::');
		// Walk from left, return the last named (non-$N) segment
		let lastNamed = '(module)';
		for (const seg of segments) {
			if (!seg.startsWith('$')) {
				lastNamed = seg;
			}
		}
		return lastNamed;
	}

	function guessScopeKind(scope: string): string {
		const lower = scope.toLowerCase();
		if (lower === '(module)' || lower === 'module') return '📦';
		if (lower.startsWith('i') && scope[0] === scope[0].toUpperCase()) return '□'; // Interface/class
		if (scope[0] === scope[0].toUpperCase()) return '□'; // PascalCase = type/class
		return 'ƒ'; // lowercase = function
	}

	interface KindChangeGroup {
		scope: string;
		changes: { vertexId: string; oldKind: string; newKind: string }[];
	}

	function groupKindChanges(changes: { vertexId: string; oldKind: string; newKind: string }[]): KindChangeGroup[] {
		const groups = new Map<string, { vertexId: string; oldKind: string; newKind: string }[]>();
		for (const kc of changes) {
			const scope = shortVertex(kc.vertexId);
			if (!groups.has(scope)) groups.set(scope, []);
			groups.get(scope)!.push(kc);
		}
		return Array.from(groups.entries())
			.map(([scope, items]) => ({ scope, changes: items }))
			.sort((a, b) => b.changes.length - a.changes.length);
	}

	// Extract search terms from a change entry to find matching code lines.
	// A change like { vertexId: "dev.cospan.repo:body.protocol", kind: "RemovedVertex" }
	// produces search terms ["protocol"] so we can find the line in the diff.
	function extractSearchTerms(change: { label: string; kind: string; vertexId?: string; src?: string; tgt?: string; name?: string }): string[] {
		const terms: string[] = [];
		// Extract the leaf name from vertexId
		if (change.vertexId) {
			const leaf = extractLeafName(change.vertexId);
			if (leaf && !leaf.startsWith('$')) terms.push(leaf);
		}
		// Edge name
		if (change.name && !change.name.startsWith('$')) terms.push(change.name);
		// Extract names from src/tgt
		if (change.src) {
			const s = extractLeafName(change.src);
			if (s && !s.startsWith('$')) terms.push(s);
		}
		if (change.tgt) {
			const t = extractLeafName(change.tgt);
			if (t && !t.startsWith('$')) terms.push(t);
		}
		// Deduplicate
		return [...new Set(terms)];
	}

	// Find diff lines that match any of the search terms.
	// Returns matching lines plus 1 line of context on each side.
	function findMatchingLines(
		hunks: DiffFile['hunks'],
		terms: string[]
	): DiffFile['hunks'][0]['lines'] {
		if (terms.length === 0) return [];
		const result: DiffFile['hunks'][0]['lines'] = [];
		for (const hunk of hunks) {
			for (let i = 0; i < hunk.lines.length; i++) {
				const line = hunk.lines[i];
				if (terms.some(t => line.content.includes(t))) {
					// Add context: 1 line before and after
					if (i > 0 && !result.includes(hunk.lines[i - 1])) {
						result.push(hunk.lines[i - 1]);
					}
					if (!result.includes(line)) {
						result.push(line);
					}
					if (i + 1 < hunk.lines.length && !result.includes(hunk.lines[i + 1])) {
						result.push(hunk.lines[i + 1]);
					}
				}
			}
		}
		return result.slice(0, 20); // Cap at 20 lines
	}

	function shortVertex(v: string): string {
		const parts = v.split('::');
		for (let i = parts.length - 1; i >= 0; i--) {
			if (!parts[i].startsWith('$')) return parts[i];
		}
		return parts[parts.length - 1] || v;
	}

	// Extract a meaningful relative path from a vertex ID, relative to its scope.
	// "src/repo.ts::createRepo::$13::$0" with scope "createRepo" → "$13.$0" (internal)
	// "dev.cospan.repo:body.description" → "description"
	function extractLeafName(v: string): string {
		if (v.includes(':') && !v.includes('::')) {
			const parts = v.split('.');
			const last = parts[parts.length - 1];
			return last.startsWith('$') ? v.split(':').pop() ?? v : last;
		}
		const parts = v.split('::');
		for (let i = parts.length - 1; i >= 0; i--) {
			if (!parts[i].startsWith('$') && parts[i] !== '') return parts[i];
		}
		return parts[parts.length - 1] || v;
	}

	// Deduplicate and summarize a list of vertex IDs within a scope.
	// Returns unique named items + a count of anonymous ones.
	function summarizeNodes(items: string[]): { named: string[]; anonymousCount: number } {
		const seen = new Set<string>();
		const named: string[] = [];
		let anonymousCount = 0;
		for (const id of items) {
			const name = extractLeafName(id);
			if (name.startsWith('$')) {
				anonymousCount++;
			} else if (!seen.has(name)) {
				seen.add(name);
				named.push(name);
			} else {
				// Duplicate named - count as internal
				anonymousCount++;
			}
		}
		return { named, anonymousCount };
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
		<span class="flex items-center gap-1 text-emerald-400">
			<span class="font-mono">+{diff.totalAdditions}</span>
		</span>
		<span class="flex items-center gap-1 text-red-400">
			<span class="font-mono">−{diff.totalDeletions}</span>
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
					<!-- Binary file: non-expandable, minimal display -->
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
							<span class="text-text-muted">{file.oldPath} → </span>
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
						<span class="shrink-0 font-mono text-xs text-red-400">−{file.deletions}</span>
					{/if}
				</button>

				<!-- Content when expanded -->
				{#if isOpen}
					<!-- Structural diff (the schema-level view) -->
					{#if sd}
						<div class="border-t border-border bg-surface-0 p-4">
							<!-- Verdict banner -->
							<div class="mb-4 flex items-center gap-3 rounded-md px-3 py-2 {sd.compatible ? 'bg-emerald-500/10' : 'bg-red-500/10'}">
								<span class="text-lg">{sd.compatible ? '✓' : '⚠'}</span>
								<div>
									<span class="text-sm font-semibold {sd.compatible ? 'text-emerald-400' : 'text-red-400'}">
										{sd.compatible ? 'Compatible change' : 'BREAKING CHANGE'}
									</span>
									<span class="ml-2 text-xs text-text-muted">
										{sd.breakingCount} breaking · {sd.nonBreakingCount} compatible ·
										{sd.oldVertexCount} → {sd.newVertexCount} vertices ·
										{sd.oldEdgeCount} → {sd.newEdgeCount} edges
									</span>
								</div>
							</div>

							<!-- Breaking changes - open by default, each clickable to show code -->
							{#if sd.breakingChanges.length > 0}
								<details open class="mb-3 rounded-md border border-red-500/20">
									<summary class="cursor-pointer px-3 py-2 text-xs font-semibold uppercase tracking-wider text-red-400 hover:bg-red-500/5">
										⚠ Breaking changes ({sd.breakingCount})
									</summary>
									<div class="space-y-0.5 px-2 pb-2">
										{#each sd.breakingChanges as bc (bc.label)}
											{@const searchTerms = extractSearchTerms(bc)}
											{@const matchingLines = findMatchingLines(file.hunks, searchTerms)}
											<details class="rounded-md">
												<summary class="flex cursor-pointer items-start gap-2 rounded-md bg-red-500/5 px-3 py-1.5 text-sm hover:bg-red-500/10 transition-colors">
													<span class="shrink-0 text-red-400 mt-0.5">⚠</span>
													<span class="text-text-primary flex-1">{bc.label}</span>
													{#if matchingLines.length > 0}
														<span class="shrink-0 text-[10px] text-text-muted mt-0.5">{matchingLines.length} lines</span>
													{/if}
													<span class="shrink-0 rounded bg-surface-2 px-1.5 py-0.5 text-[10px] text-text-muted">{bc.kind}</span>
												</summary>
												{#if matchingLines.length > 0}
													<pre class="mt-1 overflow-x-auto rounded bg-surface-0 font-mono text-[11px] leading-[18px] mx-3 mb-1">{#each matchingLines as line}<span class={lineClass(line.origin)}><span class="inline-block w-7 select-none text-right pr-1 text-[10px] text-text-muted/50">{line.oldLineno ?? ''}</span><span class="inline-block w-7 select-none text-right pr-1 text-[10px] text-text-muted/50">{line.newLineno ?? ''}</span><span class="inline-block w-3 select-none text-[10px]">{linePrefix(line.origin)}</span>{stripNewline(line.content)}
</span>{/each}</pre>
												{:else}
													<p class="mx-3 mb-1 text-[11px] text-text-muted italic">No matching code found in this diff</p>
												{/if}
											</details>
										{/each}
									</div>
								</details>
							{/if}

							<!-- Compatible changes - collapsed, each clickable to show code -->
							{#if sd.nonBreakingChanges.length > 0}
								<details class="mb-3 rounded-md border border-emerald-500/20">
									<summary class="cursor-pointer px-3 py-2 text-xs font-semibold uppercase tracking-wider text-emerald-400 hover:bg-emerald-500/5">
										✓ Compatible changes ({sd.nonBreakingCount})
									</summary>
									<div class="space-y-0.5 px-2 pb-2">
										{#each sd.nonBreakingChanges as nb (nb.label)}
											{@const searchTerms = extractSearchTerms(nb)}
											{@const matchingLines = findMatchingLines(file.hunks, searchTerms)}
											<details class="rounded-md">
												<summary class="flex cursor-pointer items-start gap-2 rounded-md bg-emerald-500/5 px-3 py-1.5 text-sm hover:bg-emerald-500/10 transition-colors">
													<span class="shrink-0 text-emerald-400 mt-0.5">✓</span>
													<span class="text-text-primary flex-1">{nb.label}</span>
													{#if matchingLines.length > 0}
														<span class="shrink-0 text-[10px] text-text-muted mt-0.5">{matchingLines.length} lines</span>
													{/if}
													<span class="shrink-0 rounded bg-surface-2 px-1.5 py-0.5 text-[10px] text-text-muted">{nb.kind}</span>
												</summary>
												{#if matchingLines.length > 0}
													<pre class="mt-1 overflow-x-auto rounded bg-surface-0 font-mono text-[11px] leading-[18px] mx-3 mb-1">{#each matchingLines as line}<span class={lineClass(line.origin)}><span class="inline-block w-7 select-none text-right pr-1 text-[10px] text-text-muted/50">{line.oldLineno ?? ''}</span><span class="inline-block w-7 select-none text-right pr-1 text-[10px] text-text-muted/50">{line.newLineno ?? ''}</span><span class="inline-block w-3 select-none text-[10px]">{linePrefix(line.origin)}</span>{stripNewline(line.content)}
</span>{/each}</pre>
												{:else}
													<p class="mx-3 mb-1 text-[11px] text-text-muted italic">No matching code found in this diff</p>
												{/if}
											</details>
										{/each}
									</div>
								</details>
							{/if}

							<!-- Structural change tree -->
							{#if sd.removedVertices.length > 0 || sd.addedVertices.length > 0 || sd.kindChanges.length > 0}
								{@const removedGroups = groupVertices(sd.removedVertices, file.path)}
								{@const addedGroups = groupVertices(sd.addedVertices, file.path)}
								<div class="mb-3 space-y-2">
									<!-- Kind changes are surfaced via the breaking/compatible
								     change classifications above. No standalone section needed. -->

									<!-- Show each scope that had changes -->
									<!-- Scope tree - each element is clickable to show its details -->
									<div class="rounded-md border border-border">
										<div class="px-3 py-2 text-xs font-semibold uppercase tracking-wider text-text-muted">
											Program elements
										</div>
										<div class="space-y-0.5 px-1 pb-1">
											{#each [...new Set([
												...removedGroups.map(g => g.scope),
												...addedGroups.map(g => g.scope),
											])].filter(s => s !== '(other)' && s !== '(module)') as scope (scope)}
												{@const removed = removedGroups.find(g => g.scope === scope)}
												{@const added = addedGroups.find(g => g.scope === scope)}
												{@const scopeKind = removed?.scopeKind ?? added?.scopeKind ?? '·'}
												{@const isNew = !removed && !!added}
												{@const isGone = !!removed && !added}
												{@const addedEdgesInScope = sd.addedEdges.filter(e => e.src.includes('::' + scope + '::') || e.src.includes('::' + scope) && !e.src.includes('::' + scope + '::'))}
												{@const removedEdgesInScope = sd.removedEdges.filter(e => e.src.includes('::' + scope + '::') || e.src.includes('::' + scope) && !e.src.includes('::' + scope + '::'))}
												<details class="rounded-md {isNew ? 'bg-emerald-500/5' : isGone ? 'bg-red-500/5' : 'bg-surface-0'}">
													<summary class="flex cursor-pointer items-center gap-2 rounded-md px-2 py-1.5 text-sm transition-colors hover:bg-surface-2">
														<span class="w-5 text-center text-base leading-none {isNew ? 'text-emerald-400' : isGone ? 'text-red-400' : 'text-amber-400'}">
															{scopeKind}
														</span>
														<span class="font-medium text-text-primary">{scope}</span>
														{#if isNew}
															<span class="rounded bg-emerald-500/15 px-1.5 py-0.5 text-[10px] font-medium text-emerald-400">added</span>
														{:else if isGone}
															<span class="rounded bg-red-500/15 px-1.5 py-0.5 text-[10px] font-medium text-red-400">removed</span>
														{:else}
															<span class="rounded bg-amber-500/15 px-1.5 py-0.5 text-[10px] font-medium text-amber-400">modified</span>
														{/if}
														<span class="ml-auto text-xs text-text-muted">
															{#if added}<span class="text-emerald-400">+{added.items.length}</span>{/if}
															{#if removed}{#if added} {/if}<span class="text-red-400">−{removed.items.length}</span>{/if}
														</span>
													</summary>
													<!-- Expanded: structural summary + relevant code -->
													<div class="border-t border-border/50">
														<!-- Structural summary -->
														<div class="px-3 py-2 text-[12px] space-y-1">
															{#if added && added.items.length > 0}
																{@const summary = summarizeNodes(added.items)}
																<div>
																	<span class="text-emerald-400 font-medium">+ {added.items.length} schema nodes added</span>
																	{#if summary.named.length > 0}
																		<span class="text-text-muted"> -</span>
																		<span class="text-text-secondary">
																			{summary.named.slice(0, 6).join(', ')}
																			{#if summary.named.length > 6}, …{/if}
																		</span>
																	{/if}
																	{#if summary.anonymousCount > 0}
																		<span class="text-text-muted"> ({summary.anonymousCount} internal)</span>
																	{/if}
																</div>
															{/if}
															{#if removed && removed.items.length > 0}
																{@const summary = summarizeNodes(removed.items)}
																<div>
																	<span class="text-red-400 font-medium">− {removed.items.length} schema nodes removed</span>
																	{#if summary.named.length > 0}
																		<span class="text-text-muted"> -</span>
																		<span class="text-text-secondary">
																			{summary.named.slice(0, 6).join(', ')}
																			{#if summary.named.length > 6}, …{/if}
																		</span>
																	{/if}
																	{#if summary.anonymousCount > 0}
																		<span class="text-text-muted"> ({summary.anonymousCount} internal)</span>
																	{/if}
																</div>
															{/if}
														</div>

														<!-- Code: the actual lines that changed in this scope -->
														{#each file.hunks.filter(h =>
															h.header.includes(scope) ||
															h.lines.some(l => l.content.includes(scope))
														) as hunk, h (h)}
															{#if h === 0}
																<div class="border-t border-border/30">
																	<div class="bg-surface-2/50 px-3 py-1 text-[10px] text-text-muted font-medium uppercase tracking-wider">
																		Code
																	</div>
																</div>
															{/if}
															<div class="border-t border-border/20">
																<div class="bg-surface-2/30 px-3 py-0.5 font-mono text-[10px] text-text-muted">
																	{hunk.header.replace(/\n$/, '')}
																</div>
																<pre class="overflow-x-auto bg-surface-0 font-mono text-[11px] leading-[18px]">{#each hunk.lines as line, l (l)}<span class={lineClass(line.origin)}><span class="inline-block w-7 select-none text-right pr-1 text-[10px] text-text-muted/50">{line.oldLineno ?? ''}</span><span class="inline-block w-7 select-none text-right pr-1 text-[10px] text-text-muted/50">{line.newLineno ?? ''}</span><span class="inline-block w-3 select-none text-[10px]">{linePrefix(line.origin)}</span>{stripNewline(line.content)}
</span>{/each}</pre>
															</div>
														{/each}
													</div>
												</details>
											{/each}
										</div>
									</div>
								</div>
							{/if}

							<!-- Collapsible raw diff -->
							{#if file.hunks.length > 0}
								<details class="mt-3 rounded-md border border-border">
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
						<!-- No structural diff - show raw textual diff as primary -->
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
				{/if}<!-- close {:else} for binary check -->
			</div>
		{/each}
	</div>
{/if}
