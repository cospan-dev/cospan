import type { PageServerLoad } from './$types';
import { getRepo } from '$lib/api/repo.js';
import { listRefs, getObject } from '$lib/api/node.js';
import { createHighlighter } from 'shiki';
import type { NodeRef, NodeObject } from '$lib/api/node.js';

// Map file extensions to Shiki language identifiers
const extensionToLang: Record<string, string> = {
	ts: 'typescript',
	tsx: 'tsx',
	js: 'javascript',
	jsx: 'jsx',
	rs: 'rust',
	py: 'python',
	go: 'go',
	json: 'json',
	yaml: 'yaml',
	yml: 'yaml',
	toml: 'toml',
	md: 'markdown',
	css: 'css',
	html: 'html',
	sql: 'sql',
	proto: 'proto',
	graphql: 'graphql',
	sh: 'bash',
	bash: 'bash',
	txt: 'text',
	xml: 'xml',
	svg: 'xml',
	c: 'c',
	cpp: 'cpp',
	h: 'c',
	hpp: 'cpp',
	java: 'java',
	kt: 'kotlin',
	swift: 'swift',
	rb: 'ruby',
	php: 'php',
	zig: 'zig'
};

function detectLanguage(path: string): string {
	const ext = path.split('.').pop()?.toLowerCase() ?? '';
	return extensionToLang[ext] ?? 'text';
}

// Cached Shiki highlighter instance
let highlighterPromise: ReturnType<typeof createHighlighter> | null = null;

function getHighlighter() {
	if (!highlighterPromise) {
		highlighterPromise = createHighlighter({
			themes: ['github-dark'],
			langs: [
				'typescript',
				'tsx',
				'javascript',
				'jsx',
				'rust',
				'python',
				'go',
				'json',
				'yaml',
				'toml',
				'markdown',
				'css',
				'html',
				'sql',
				'bash',
				'text',
				'xml',
				'c',
				'cpp',
				'java'
			]
		});
	}
	return highlighterPromise;
}

interface TreePageData {
	repo: {
		did: string;
		name: string;
		protocol: string;
		starCount: number;
		openIssueCount: number;
		openMrCount: number;
		description: string | null;
		createdAt: string;
		updatedAt: string;
	};
	path: string;
	mode: 'tree' | 'blob';
	refs?: NodeRef[];
	object?: {
		code: string;
		language: string;
		highlightedHtml: string;
	};
	error?: string;
}

export const load: PageServerLoad = async ({ params }): Promise<TreePageData> => {
	const repo = await getRepo({ did: params.did, name: params.repo });
	const path = params.path || '';

	// If we have a nodeUrl on the repo, use it to fetch real data.
	// For now, we try to list refs and show the tree.
	const nodeUrl = (repo as unknown as Record<string, unknown>).nodeUrl as string | undefined;

	// If no path provided, show the refs tree
	if (!path) {
		if (nodeUrl) {
			try {
				const refList = await listRefs(nodeUrl, params.did, params.repo);
				return {
					repo,
					path: '',
					mode: 'tree',
					refs: refList.refs
				};
			} catch (e) {
				return {
					repo,
					path: '',
					mode: 'tree',
					refs: [],
					error: `Could not fetch refs from node: ${e instanceof Error ? e.message : 'Unknown error'}`
				};
			}
		}

		// No nodeUrl; return empty ref list (placeholder mode)
		return {
			repo,
			path: '',
			mode: 'tree',
			refs: []
		};
	}

	// Path provided: try to fetch the object if we have a nodeUrl
	if (nodeUrl) {
		try {
			const obj: NodeObject = await getObject(nodeUrl, params.did, params.repo, path);
			const language = detectLanguage(path);

			let highlightedHtml: string;
			try {
				const highlighter = await getHighlighter();
				highlightedHtml = highlighter.codeToHtml(obj.data, {
					lang: language,
					theme: 'github-dark'
				});
			} catch {
				// Fall back to plain text if highlighting fails for this language
				highlightedHtml = `<pre><code>${escapeHtml(obj.data)}</code></pre>`;
			}

			return {
				repo,
				path,
				mode: 'blob',
				object: {
					code: obj.data,
					language,
					highlightedHtml
				}
			};
		} catch (e) {
			// If object fetch fails, fall back to tree view for this path
			try {
				const refList = await listRefs(nodeUrl, params.did, params.repo);
				return {
					repo,
					path,
					mode: 'tree',
					refs: refList.refs,
					error: `Could not fetch object "${path}": ${e instanceof Error ? e.message : 'Unknown error'}`
				};
			} catch {
				return {
					repo,
					path,
					mode: 'tree',
					refs: [],
					error: `Could not connect to node`
				};
			}
		}
	}

	// No nodeUrl: show placeholder tree with the path as context
	return {
		repo,
		path,
		mode: 'tree',
		refs: [],
		error: 'No node URL configured for this repository. Code browsing is unavailable.'
	};
};

function escapeHtml(text: string): string {
	return text
		.replace(/&/g, '&amp;')
		.replace(/</g, '&lt;')
		.replace(/>/g, '&gt;')
		.replace(/"/g, '&quot;');
}
