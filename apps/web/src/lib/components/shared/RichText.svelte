<script lang="ts" module>
	// Load the relationaltext → HTML lens rules at module level.
	// This is a panproto lens that maps feature names to HTML element names.
	import lensRules from '$lib/data/relationaltext-to-html.lens.json';

	type LensRule = { match: { name: string }; replace: { name: string | { template: string }; renameAttrs?: Record<string, string>; dropAttrs?: string[] } | null };

	// Build a lookup: feature name → HTML element name + attr transforms
	const featureToElement: Record<string, { tag: string; renameAttrs?: Record<string, string>; dropAttrs?: string[] }> = {};
	for (const rule of (lensRules as { rules: LensRule[] }).rules) {
		if (!rule.replace) continue;
		const tag = typeof rule.replace.name === 'string'
			? rule.replace.name
			: rule.replace.name.template; // e.g., "h{level}"
		featureToElement[rule.match.name] = {
			tag,
			renameAttrs: rule.replace.renameAttrs,
			dropAttrs: rule.replace.dropAttrs,
		};
	}

	// Map marked.js token types to relationaltext feature names
	const tokenToFeature: Record<string, string> = {
		paragraph: 'paragraph',
		heading: 'heading',
		code: 'code-block',
		blockquote: 'blockquote',
		hr: 'horizontal-rule',
		strong: 'bold',
		em: 'italic',
		codespan: 'code',
		del: 'strikethrough',
		link: 'link',
		image: 'image',
		br: 'line-break',
	};

	function resolveTag(featureName: string, attrs?: Record<string, unknown>): string {
		const mapping = featureToElement[featureName];
		if (!mapping) return 'span';
		if (mapping.tag.includes('{')) {
			// Template: "h{level}" → "h1", "h2", etc.
			return mapping.tag.replace(/\{(\w+)\}/g, (_, key) => String(attrs?.[key] ?? ''));
		}
		return mapping.tag;
	}
</script>

<script lang="ts">
	import { marked } from 'marked';

	let { text }: { text: string | null } = $props();

	let tokens = $derived(text ? marked.lexer(text) : []);
</script>

{#if tokens.length === 0}
	<span class="text-text-secondary">No content.</span>
{:else}
	<div class="prose-rt">
		{#each tokens as token}
			{@render blockToken(token)}
		{/each}
	</div>
{/if}

{#snippet blockToken(token: marked.Token)}
	{@const feature = tokenToFeature[token.type]}
	{@const tag = feature ? resolveTag(feature, 'depth' in token ? { level: token.depth } : {}) : ''}
	{#if token.type === 'paragraph'}
		<p>{@render inlineTokens(token.tokens ?? [])}</p>
	{:else if token.type === 'heading'}
		<svelte:element this={tag}>{@render inlineTokens(token.tokens ?? [])}</svelte:element>
	{:else if token.type === 'code'}
		<pre><code>{token.text}</code></pre>
	{:else if token.type === 'blockquote'}
		<blockquote>
			{#each token.tokens ?? [] as child}
				{@render blockToken(child)}
			{/each}
		</blockquote>
	{:else if token.type === 'list'}
		<svelte:element this={token.ordered ? 'ol' : 'ul'}>
			{#each token.items as item}
				<li>
					{#each item.tokens ?? [] as child}
						{#if child.type === 'text' && 'tokens' in child}
							{@render inlineTokens(child.tokens ?? [])}
						{:else}
							{@render blockToken(child)}
						{/if}
					{/each}
				</li>
			{/each}
		</svelte:element>
	{:else if token.type === 'hr'}
		<hr />
	{:else if token.type === 'space'}
		<!-- skip -->
	{:else if token.type === 'text'}
		{#if 'tokens' in token && token.tokens}
			<p>{@render inlineTokens(token.tokens)}</p>
		{:else}
			<p>{token.text}</p>
		{/if}
	{/if}
{/snippet}

{#snippet inlineTokens(tokens: marked.Token[])}
	{#each tokens as token}
		{@render inlineToken(token)}
	{/each}
{/snippet}

{#snippet inlineToken(token: marked.Token)}
	{@const feature = tokenToFeature[token.type]}
	{@const tag = feature ? resolveTag(feature) : ''}
	{#if token.type === 'text'}
		{token.text}
	{:else if token.type === 'strong'}
		<strong>{@render inlineTokens(token.tokens ?? [])}</strong>
	{:else if token.type === 'em'}
		<em>{@render inlineTokens(token.tokens ?? [])}</em>
	{:else if token.type === 'codespan'}
		<code>{token.text}</code>
	{:else if token.type === 'del'}
		<s>{@render inlineTokens(token.tokens ?? [])}</s>
	{:else if token.type === 'link'}
		<a href={token.href} target="_blank" rel="noopener">{@render inlineTokens(token.tokens ?? [])}</a>
	{:else if token.type === 'image'}
		<img src={token.href} alt={token.text} />
	{:else if token.type === 'br'}
		<br />
	{:else if token.type === 'escape'}
		{token.text}
	{:else if token.type === 'paragraph'}
		{@render inlineTokens(token.tokens ?? [])}
	{/if}
{/snippet}

<style>
	.prose-rt :global(p) { margin-bottom: 0.75rem; line-height: 1.625; }
	.prose-rt :global(h1) { margin-bottom: 0.75rem; font-size: 1.25rem; font-weight: 600; }
	.prose-rt :global(h2) { margin-bottom: 0.75rem; font-size: 1.125rem; font-weight: 600; }
	.prose-rt :global(h3) { margin-bottom: 0.5rem; font-size: 1rem; font-weight: 600; }
	.prose-rt :global(h4) { margin-bottom: 0.5rem; font-size: 0.875rem; font-weight: 600; }
	.prose-rt :global(pre) { margin-bottom: 0.75rem; overflow-x: auto; border-radius: 0.5rem; padding: 1rem; font-size: 0.75rem; font-family: var(--font-mono); background: var(--color-surface-2, #1a1b2e); }
	.prose-rt :global(code) { border-radius: 0.25rem; padding: 0.125rem 0.375rem; font-size: 0.75rem; font-family: var(--font-mono); background: var(--color-surface-2, #1a1b2e); }
	.prose-rt :global(pre code) { padding: 0; background: none; }
	.prose-rt :global(blockquote) { margin-bottom: 0.75rem; border-left: 2px solid var(--color-border, #333); padding-left: 1rem; font-style: italic; opacity: 0.85; }
	.prose-rt :global(ul) { margin-bottom: 0.75rem; padding-left: 1.5rem; list-style: disc; }
	.prose-rt :global(ol) { margin-bottom: 0.75rem; padding-left: 1.5rem; list-style: decimal; }
	.prose-rt :global(li) { margin-bottom: 0.25rem; }
	.prose-rt :global(hr) { margin: 1rem 0; border-color: var(--color-border, #333); }
	.prose-rt :global(a) { color: var(--color-accent, #7c8aff); text-decoration: underline; }
	.prose-rt :global(a:hover) { color: var(--color-accent-hover, #9ba5ff); }
	.prose-rt :global(img) { max-width: 100%; border-radius: 0.5rem; }
	.prose-rt :global(strong) { font-weight: 600; }
</style>
