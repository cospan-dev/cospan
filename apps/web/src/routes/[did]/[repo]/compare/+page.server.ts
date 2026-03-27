import { env } from '$env/dynamic/private';
import { getRepo } from '$lib/api/repo.js';
import { listRefs } from '$lib/api/node.js';
import type { NodeRef } from '$lib/api/node.js';

const DEFAULT_NODE_URL = env.NODE_URL ?? 'http://localhost:3002';

export const load = async ({ params, url }: { params: { did: string; repo: string }; url: URL }) => {
	const baseRef = url.searchParams.get('base') ?? '';
	const headRef = url.searchParams.get('head') ?? '';

	let repo = null;
	let refs: NodeRef[] = [];

	try {
		repo = await getRepo({ did: params.did, name: params.repo });
	} catch {
		// Repo metadata unavailable.
	}

	try {
		const result = await listRefs(DEFAULT_NODE_URL, params.did, params.repo);
		refs = result.refs;
	} catch {
		// Node might be unreachable.
	}

	return {
		did: params.did,
		repoName: params.repo,
		repo,
		refs,
		baseRef,
		headRef,
	};
};
