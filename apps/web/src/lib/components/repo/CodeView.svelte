<script lang="ts">
	let {
		code,
		language,
		filePath,
		highlightedHtml
	}: {
		code: string;
		language: string;
		filePath: string;
		highlightedHtml: string;
	} = $props();

	let pathSegments = $derived(filePath.split('/').filter(Boolean));
	let lineCount = $derived(code.split('\n').length);

	let languageLabels: Record<string, string> = {
		typescript: 'TypeScript',
		javascript: 'JavaScript',
		rust: 'Rust',
		python: 'Python',
		go: 'Go',
		json: 'JSON',
		yaml: 'YAML',
		toml: 'TOML',
		markdown: 'Markdown',
		css: 'CSS',
		html: 'HTML',
		sql: 'SQL',
		protobuf: 'Protobuf',
		graphql: 'GraphQL'
	};

	let displayLanguage = $derived(languageLabels[language] ?? language);
</script>

<div class="rounded-lg border border-border bg-surface-1 overflow-hidden">
	<!-- File header with breadcrumb and language badge -->
	<div class="flex items-center justify-between border-b border-border px-4 py-2">
		<nav class="flex items-center gap-1 text-sm min-w-0">
			{#each pathSegments as segment, i}
				{#if i > 0}
					<span class="text-text-secondary">/</span>
				{/if}
				{#if i === pathSegments.length - 1}
					<span class="font-medium text-text-primary truncate">{segment}</span>
				{:else}
					<span class="text-text-secondary truncate">{segment}</span>
				{/if}
			{/each}
		</nav>
		<div class="flex items-center gap-3 shrink-0">
			<span class="text-xs text-text-secondary">{lineCount} lines</span>
			<span class="rounded-full bg-surface-2 px-2 py-0.5 font-mono text-xs text-text-secondary">
				{displayLanguage}
			</span>
		</div>
	</div>

	<!-- Syntax-highlighted code display -->
	<div class="overflow-x-auto">
		<div class="code-view font-mono text-sm leading-relaxed">
			{@html highlightedHtml}
		</div>
	</div>
</div>

<style>
	.code-view :global(pre) {
		margin: 0;
		padding: 1rem;
		background: transparent !important;
		overflow-x: auto;
	}

	.code-view :global(code) {
		font-family: 'JetBrains Mono', 'Fira Code', monospace;
		font-size: 0.8125rem;
		line-height: 1.6;
	}

	.code-view :global(.line) {
		display: inline-block;
		width: 100%;
	}

	.code-view :global(.line::before) {
		content: attr(data-line);
		display: inline-block;
		width: 3em;
		margin-right: 1.5em;
		text-align: right;
		color: oklch(0.40 0.01 260);
		user-select: none;
	}
</style>
