<script lang="ts">
	import type { ProjectSchemaResponse, CommitSchemaStatsResponse, ImportStatusResponse } from '$lib/api/schema.js';

	let {
		projectSchema,
		schemaStats,
		importStatus,
	}: {
		projectSchema: ProjectSchemaResponse | null;
		schemaStats: CommitSchemaStatsResponse | null;
		importStatus: ImportStatusResponse | null;
	} = $props();

	// Language hue mapping (same as RepoCard protocol hues)
	function languageHue(name: string): number {
		const map: Record<string, number> = {
			typescript: 230, javascript: 50, rust: 25, python: 210,
			go: 170, java: 30, ruby: 350, 'c++': 200, c: 200,
			swift: 15, kotlin: 280, scala: 5, 'atproto-lexicon': 195,
			json: 45, yaml: 80, toml: 35, css: 265, html: 15,
			svelte: 12, vue: 150, jsx: 50, tsx: 230, sql: 180,
			shell: 120, markdown: 100, avro: 40, openapi: 190,
		};
		return map[name.toLowerCase()] ?? 260;
	}

	// Count recent breaking changes
	let recentBreaking = $derived(
		schemaStats?.commits
			?.slice(0, 10)
			.reduce((sum, c) => sum + c.breakingChangeCount, 0) ?? 0
	);
</script>

{#if importStatus && !importStatus.ready}
	<div class="mb-4 rounded-lg border border-amber-500/30 bg-amber-500/5 p-4">
		<div class="flex items-center gap-3">
			<div class="h-4 w-4 animate-spin rounded-full border-2 border-amber-400/30 border-t-amber-400"></div>
			<div>
				<span class="text-sm font-medium text-amber-400">Schema analysis in progress</span>
				<p class="mt-0.5 text-xs text-text-muted">
					panproto is parsing your repository. Structural data (language breakdown, breaking change detection, schema graphs) will appear once processing completes. This can take a few minutes for large repositories.
				</p>
			</div>
		</div>
	</div>
{/if}

{#if projectSchema || schemaStats}
	<div class="mb-4 rounded-lg border border-border bg-surface-1 p-4">
		<h3 class="mb-3 text-xs font-semibold uppercase tracking-wider text-text-muted">
			Schema Overview
		</h3>

		{#if projectSchema}
			<!-- Languages detected -->
			<div class="mb-3 flex flex-wrap gap-1.5">
				{#each projectSchema.languages as lang (lang.name)}
					<span
						class="inline-flex items-center gap-1.5 rounded-full px-2.5 py-1 text-[11px] font-medium"
						style="background: oklch(0.25 0.04 {languageHue(lang.name)}); color: oklch(0.75 0.12 {languageHue(lang.name)});"
					>
						<span
							class="h-1.5 w-1.5 rounded-full"
							style="background: oklch(0.65 0.16 {languageHue(lang.name)});"
						></span>
						{lang.name}
						<span class="opacity-60">{lang.fileCount}</span>
					</span>
				{/each}
			</div>

			<!-- Schema statistics -->
			<div class="mb-3 flex items-center gap-4 text-sm text-text-secondary">
				<span>
					<span class="font-medium text-text-primary">{projectSchema.totalVertexCount.toLocaleString()}</span>
					schema elements
				</span>
				<span class="text-text-muted">&middot;</span>
				<span>
					<span class="font-medium text-text-primary">{projectSchema.totalEdgeCount.toLocaleString()}</span>
					edges
				</span>
				<span class="text-text-muted">&middot;</span>
				<span>
					<span class="font-medium text-text-primary">{projectSchema.parsedFileCount}</span>/{projectSchema.fileCount} files parsed
				</span>
			</div>
		{/if}

		<!-- Breaking change trend -->
		{#if schemaStats && schemaStats.commits.length > 0}
			<div class="flex items-center gap-2 text-xs">
				{#if recentBreaking === 0}
					<span class="inline-flex items-center gap-1.5 rounded-md bg-emerald-500/10 px-2 py-1 font-medium text-emerald-400">
						<svg class="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2.5">
							<path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7" />
						</svg>
						No breaking changes in the last {Math.min(10, schemaStats.commits.length)} commits
					</span>
				{:else}
					<span class="inline-flex items-center gap-1.5 rounded-md bg-red-500/10 px-2 py-1 font-medium text-red-400">
						<svg class="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2.5">
							<path stroke-linecap="round" stroke-linejoin="round" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
						</svg>
						{recentBreaking} breaking {recentBreaking === 1 ? 'change' : 'changes'} in the last {Math.min(10, schemaStats.commits.length)} commits
					</span>
				{/if}
			</div>
		{/if}
	</div>
{/if}
