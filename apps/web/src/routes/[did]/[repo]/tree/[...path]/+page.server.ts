import type { PageServerLoad } from './$types';
import { getRepo } from '$lib/api/repo.js';
import { xrpcQuery } from '$lib/api/client.js';
import { getFileSchema, type FileSchemaResponse } from '$lib/api/schema.js';
import { createHighlighter } from 'shiki';

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

// Appview proxy response types
interface ProxyRef {
	ref: string;
	target: string;
}

interface ProxyRefList {
	refs: ProxyRef[];
}

interface ProxyObject {
	id: string;
	object: {
		type: string;
		protocol?: string;
		vertexCount?: number;
		edgeCount?: number;
		message?: string;
		author?: string;
		[key: string]: unknown;
	};
}

// Frontend types
interface DisplayRef {
	name: string;
	target: string;
	type: 'branch' | 'tag';
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
	refs?: DisplayRef[];
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

	// If no path provided, show the refs tree via appview proxy
	if (!path) {
		try {
			const result = await xrpcQuery<ProxyRefList>(
				'dev.cospan.node.proxy.listRefs',
				{ did: params.did, repo: params.repo }
			);
			const refs: DisplayRef[] = result.refs.map((r) => ({
				name: r.ref,
				target: r.target,
				type: r.ref.startsWith('refs/tags/') ? 'tag' as const : 'branch' as const,
			}));
			return { repo, path: '', mode: 'tree', refs };
		} catch (e) {
			return {
				repo,
				path: '',
				mode: 'tree',
				refs: [],
				error: `Could not fetch refs: ${e instanceof Error ? e.message : 'Unknown error'}`
			};
		}
	}

	// Path provided: try to fetch the blob via appview proxy.
	// The proxy getObject endpoint returns structured metadata, but for
	// code display we need raw file content. Use the node directly for
	// blob fetching via the git mirror's listCommits to get the tree.
	// For now, try fetching from the node's git smart HTTP endpoint
	// which serves raw blobs.
	try {
		// Use the appview proxy to get the object
		const result = await xrpcQuery<ProxyObject>(
			'dev.cospan.node.proxy.getObject',
			{ did: params.did, repo: params.repo, id: path }
		);

		// The proxy returns structured data, not raw file content.
		// We need to extract the content if it's a schema object,
		// or show metadata for other types.
		const language = detectLanguage(path);
		const code = JSON.stringify(result.object, null, 2);

		let highlightedHtml: string;
		try {
			const highlighter = await getHighlighter();
			highlightedHtml = highlighter.codeToHtml(code, {
				lang: 'json',
				theme: 'github-dark'
			});
		} catch {
			highlightedHtml = `<pre><code>${escapeHtml(code)}</code></pre>`;
		}

		// Fetch file schema (best-effort)
		let fileSchema: FileSchemaResponse | null = null;
		try {
			fileSchema = await getFileSchema({
				did: params.did,
				repo: params.repo,
				commit: 'HEAD',
				path,
			});
		} catch {
			// Schema unavailable
		}

		return {
			repo,
			path,
			mode: 'blob',
			object: { code, language, highlightedHtml },
			fileSchema,
		};
	} catch (e) {
		// Object not found or node unreachable
		try {
			const result = await xrpcQuery<ProxyRefList>(
				'dev.cospan.node.proxy.listRefs',
				{ did: params.did, repo: params.repo }
			);
			const refs: DisplayRef[] = result.refs.map((r) => ({
				name: r.ref,
				target: r.target,
				type: r.ref.startsWith('refs/tags/') ? 'tag' as const : 'branch' as const,
			}));
			return {
				repo,
				path,
				mode: 'tree',
				refs,
				error: `Could not fetch "${path}": ${e instanceof Error ? e.message : 'Unknown error'}`
			};
		} catch {
			return {
				repo,
				path,
				mode: 'tree',
				refs: [],
				error: 'Could not connect to node'
			};
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
