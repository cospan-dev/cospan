import type { PageServerLoad } from './$types';
import { env } from '$env/dynamic/private';
import { getRepo } from '$lib/api/repo.js';
import { listRefs, getObject } from '$lib/api/node.js';
import { getFileSchema, type FileSchemaResponse } from '$lib/api/schema.js';
import { createHighlighter } from 'shiki';
import type { NodeRef, NodeObject } from '$lib/api/node.js';

const DEFAULT_NODE_URL = env.NODE_URL ?? 'http://localhost:3002';

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
				'java',
				'kotlin',
				'swift',
				'ruby',
				'php',
				'zig',
				'graphql',
				'proto',
				'svelte',
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
	fileSchema?: FileSchemaResponse | null;
	error?: string;
}

export const load: PageServerLoad = async ({ params }): Promise<TreePageData> => {
	const repo = await getRepo({ did: params.did, name: params.repo });
	const path = params.path || '';

	const nodeUrl = (repo as unknown as Record<string, unknown>).nodeUrl as string | undefined
		?? DEFAULT_NODE_URL;

	// If no path provided, show the refs tree
	if (!path) {
		{
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
	}

	// Path provided: try to fetch the object from the node.
	{
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

			// Fetch file schema in parallel (best-effort)
			let fileSchema: FileSchemaResponse | null = null;
			try {
				fileSchema = await getFileSchema({
					did: params.did,
					repo: params.repo,
					commit: 'HEAD',
					path,
				});
			} catch {
				// Schema unavailable; sidebar won't appear
			}

			return {
				repo,
				path,
				mode: 'blob',
				object: {
					code: obj.data,
					language,
					highlightedHtml
				},
				fileSchema,
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

};

function escapeHtml(text: string): string {
	return text
		.replace(/&/g, '&amp;')
		.replace(/</g, '&lt;')
		.replace(/>/g, '&gt;')
		.replace(/"/g, '&quot;');
}
