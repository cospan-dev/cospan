import type { PageServerLoad } from './$types';
import { getRepo } from '$lib/api/repo.js';
import { xrpcQuery } from '$lib/api/client.js';
import { getFileSchema, type FileSchemaResponse } from '$lib/api/schema.js';
import { createHighlighter } from 'shiki';

// Map file extensions to Shiki language identifiers
const extensionToLang: Record<string, string> = {
	ts: 'typescript', tsx: 'tsx', js: 'javascript', jsx: 'jsx',
	rs: 'rust', py: 'python', go: 'go', json: 'json',
	yaml: 'yaml', yml: 'yaml', toml: 'toml', md: 'markdown',
	css: 'css', html: 'html', sql: 'sql', proto: 'proto',
	graphql: 'graphql', sh: 'bash', bash: 'bash', txt: 'text',
	xml: 'xml', svg: 'xml', c: 'c', cpp: 'cpp', h: 'c', hpp: 'cpp',
	java: 'java', kt: 'kotlin', swift: 'swift', rb: 'ruby',
	php: 'php', zig: 'zig', svelte: 'svelte',
};

function detectLanguage(path: string): string {
	const ext = path.split('.').pop()?.toLowerCase() ?? '';
	return extensionToLang[ext] ?? 'text';
}

let highlighterPromise: ReturnType<typeof createHighlighter> | null = null;
function getHighlighter() {
	if (!highlighterPromise) {
		highlighterPromise = createHighlighter({
			themes: ['github-dark'],
			langs: [
				'typescript', 'tsx', 'javascript', 'jsx', 'rust', 'python',
				'go', 'json', 'yaml', 'toml', 'markdown', 'css', 'html',
				'sql', 'bash', 'text', 'xml', 'c', 'cpp', 'java', 'kotlin',
				'swift', 'ruby', 'php', 'zig', 'graphql', 'proto', 'svelte',
			]
		});
	}
	return highlighterPromise;
}

// XRPC response types
interface TreeEntry {
	name: string;
	type: 'file' | 'dir';
	oid: string;
	size?: number;
}

interface ListTreeResponse {
	ref: string;
	commit: string;
	path: string;
	entries: TreeEntry[];
}

interface GetBlobResponse {
	path: string;
	commit: string;
	binary: boolean;
	size: number;
	content: string | null;
}

export const load: PageServerLoad = async ({ params }) => {
	const repo = await getRepo({ did: params.did, name: params.repo });
	const path = params.path || '';

	// Determine if the path starts with a ref name (refs/heads/*, refs/tags/*)
	// and split it into ref + subpath
	let refName: string | undefined;
	let treePath = path;

	if (path.startsWith('refs/heads/') || path.startsWith('refs/tags/')) {
		// Extract the ref name and any remaining path
		const parts = path.split('/');
		// refs/heads/main or refs/tags/v1.0
		refName = parts.slice(0, 3).join('/');
		treePath = parts.slice(3).join('/');
	}

	// If no path (or just a ref with no subpath), show directory listing
	if (!treePath) {
		try {
			const result = await xrpcQuery<ListTreeResponse>(
				'dev.panproto.node.proxy.listTree',
				{
					did: params.did,
					repo: params.repo,
					ref: refName,
					path: '',
				}
			);
			return {
				repo, path, mode: 'tree' as const,
				ref: refName ?? result.ref,
				entries: result.entries,
			};
		} catch (e) {
			return {
				repo, path, mode: 'tree' as const,
				ref: refName,
				entries: [] as TreeEntry[],
				error: `Could not list tree: ${e instanceof Error ? e.message : 'Unknown error'}`,
			};
		}
	}

	// Path provided: try to list it as a directory first, then as a blob
	try {
		const result = await xrpcQuery<ListTreeResponse>(
			'dev.panproto.node.proxy.listTree',
			{
				did: params.did,
				repo: params.repo,
				ref: refName,
				path: treePath,
			}
		);
		return {
			repo, path, mode: 'tree' as const,
			ref: refName ?? result.ref,
			entries: result.entries,
		};
	} catch {
		// Not a directory, try as a file
	}

	try {
		const blob = await xrpcQuery<GetBlobResponse>(
			'dev.panproto.node.proxy.getBlob',
			{
				did: params.did,
				repo: params.repo,
				ref: refName,
				path: treePath,
			}
		);

		if (blob.binary || !blob.content) {
			return {
				repo, path, mode: 'blob' as const,
				ref: refName,
				object: {
					code: '(binary file)',
					language: 'text',
					highlightedHtml: '<pre><code>(binary file)</code></pre>',
				},
			};
		}

		const language = detectLanguage(treePath);
		let highlightedHtml: string;
		try {
			const highlighter = await getHighlighter();
			highlightedHtml = highlighter.codeToHtml(blob.content, {
				lang: language,
				theme: 'github-dark'
			});
		} catch {
			highlightedHtml = `<pre><code>${escapeHtml(blob.content)}</code></pre>`;
		}

		// Fetch file schema (best-effort)
		let fileSchema: FileSchemaResponse | null = null;
		try {
			fileSchema = await getFileSchema({
				did: params.did,
				repo: params.repo,
				commit: blob.commit,
				path: treePath,
			});
		} catch { /* schema unavailable */ }

		return {
			repo, path, mode: 'blob' as const,
			ref: refName,
			object: { code: blob.content, language, highlightedHtml },
			fileSchema,
		};
	} catch (e) {
		return {
			repo, path, mode: 'tree' as const,
			ref: refName,
			entries: [] as TreeEntry[],
			error: `Could not fetch "${treePath}": ${e instanceof Error ? e.message : 'Unknown error'}`,
		};
	}
};

function escapeHtml(text: string): string {
	return text
		.replace(/&/g, '&amp;')
		.replace(/</g, '&lt;')
		.replace(/>/g, '&gt;')
		.replace(/"/g, '&quot;');
}
