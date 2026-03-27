import type { PageServerLoad } from './$types';
import { env } from '$env/dynamic/private';
import { getRepo } from '$lib/api/repo.js';
import { listRefs } from '$lib/api/node.js';

const DEFAULT_NODE_URL = env.NODE_URL ?? 'http://localhost:3002';

export const load: PageServerLoad = async ({ params }) => {
	let repo = null;
	let branches: { name: string; target: string }[] = [];

	try {
		repo = await getRepo({ did: params.did, name: params.repo });
	} catch {
		// Repo metadata unavailable.
	}

	try {
		const result = await listRefs(DEFAULT_NODE_URL, params.did, params.repo);
		branches = result.refs
			.filter((r) => r.type === 'branch')
			.map((r) => ({ name: r.name, target: r.target }));
	} catch {
		// Node might be unreachable; return empty list.
	}

	return {
		did: params.did,
		repoName: params.repo,
		repo,
		branches,
	};
};
