<script lang="ts">
	import type { FileSchemaResponse } from '$lib/api/schema.js';

	let {
		fileSchema,
	}: {
		fileSchema: FileSchemaResponse | null;
	} = $props();

	// Group vertices by kind
	interface KindGroup {
		kind: string;
		icon: string;
		items: { name: string; humanLabel: string }[];
	}

	function kindIcon(kind: string): string {
		const lower = kind.toLowerCase();
		if (lower.includes('function') || lower.includes('method')) return 'f';
		if (lower.includes('class') || lower.includes('struct') || lower.includes('interface') || lower.includes('record') || lower.includes('type') || lower.includes('object')) return 'T';
		if (lower.includes('field') || lower.includes('property') || lower.includes('member')) return '#';
		if (lower.includes('enum') || lower.includes('variant')) return 'E';
		if (lower.includes('module') || lower.includes('namespace')) return 'M';
		if (lower.includes('import') || lower.includes('export')) return '>';
		return '.';
	}

	let groups = $derived<KindGroup[]>(() => {
		if (!fileSchema || fileSchema.vertices.length === 0) return [];
		const map = new Map<string, { name: string; humanLabel: string }[]>();
		for (const v of fileSchema.vertices) {
			if (!map.has(v.kind)) map.set(v.kind, []);
			map.get(v.kind)!.push({ name: v.name, humanLabel: v.humanLabel });
		}
		return Array.from(map.entries())
			.map(([kind, items]) => ({
				kind,
				icon: kindIcon(kind),
				items: items.sort((a, b) => a.name.localeCompare(b.name)),
			}))
			.sort((a, b) => b.items.length - a.items.length);
	});

	// Language hue (same mapping as SchemaHealthCard)
	function languageHue(name: string): number {
		const map: Record<string, number> = {
			typescript: 230, javascript: 50, rust: 25, python: 210,
			go: 170, java: 30, ruby: 350, svelte: 12, json: 45,
		};
		return map[name?.toLowerCase()] ?? 260;
	}

	let collapsed = $state(new Set<string>());
	function toggleKind(kind: string) {
		const next = new Set(collapsed);
		if (next.has(kind)) next.delete(kind);
		else next.add(kind);
		collapsed = next;
	}
</script>

{#if fileSchema && fileSchema.vertices.length > 0}
	<div class="rounded-lg border border-border bg-surface-1 p-3">
		<!-- Language badge -->
		{#if fileSchema.language}
			{@const hue = languageHue(fileSchema.language)}
			<div class="mb-3 flex items-center gap-2">
				<span
					class="inline-flex items-center gap-1.5 rounded-full px-2 py-0.5 text-[10px] font-medium"
					style="background: oklch(0.25 0.04 {hue}); color: oklch(0.75 0.12 {hue});"
				>
					<span class="h-1.5 w-1.5 rounded-full" style="background: oklch(0.65 0.16 {hue});"></span>
					{fileSchema.language}
				</span>
				<span class="text-[10px] text-text-muted">
					{fileSchema.vertexCount} elements &middot; {fileSchema.edgeCount} edges
				</span>
			</div>
		{/if}

		<!-- Schema outline grouped by kind -->
		<div class="max-h-[60vh] space-y-2 overflow-y-auto">
			{#each groups() as group (group.kind)}
				<div>
					<button
						type="button"
						class="flex w-full items-center gap-1.5 text-[11px] font-medium text-text-muted hover:text-text-secondary transition-colors"
						onclick={() => toggleKind(group.kind)}
					>
						<span class="w-4 text-center font-mono text-[10px] text-text-muted">{group.icon}</span>
						<span class="uppercase tracking-wider">{group.kind}</span>
						<span class="ml-auto text-[10px] opacity-60">{group.items.length}</span>
						<svg
							class="h-3 w-3 transition-transform {collapsed.has(group.kind) ? '' : 'rotate-90'}"
							fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"
						>
							<path stroke-linecap="round" stroke-linejoin="round" d="M9 5l7 7-7 7" />
						</svg>
					</button>
					{#if !collapsed.has(group.kind)}
						<ul class="mt-0.5 space-y-px pl-5">
							{#each group.items.slice(0, 20) as item (item.name)}
								<li class="truncate text-[11px] text-text-secondary" title={item.humanLabel}>
									{item.name}
								</li>
							{/each}
							{#if group.items.length > 20}
								<li class="text-[10px] text-text-muted italic">
									... {group.items.length - 20} more
								</li>
							{/if}
						</ul>
					{/if}
				</div>
			{/each}
		</div>
	</div>
{:else if fileSchema}
	<div class="rounded-lg border border-border bg-surface-1 p-3 text-xs text-text-muted">
		No parseable schema elements
	</div>
{/if}
