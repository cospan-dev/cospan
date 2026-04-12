import type { PageServerLoad } from './$types';
import { env } from '$env/dynamic/private';
import { getRepo } from '$lib/api/repo.js';
import { listRefs } from '$lib/api/node.js';
import {
	compareBranchSchemas,
	type BranchComparisonResponse,
} from '$lib/api/schema.js';

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

	// Compare each non-default branch against the default branch.
	const defaultBranch = repo?.defaultBranch ?? 'main';
	const comparisons: Record<string, BranchComparisonResponse | null> = {};

	if (branches.length > 1 && repo?.source !== 'tangled') {
		const nonDefault = branches.filter((b) => b.name !== defaultBranch);
		await Promise.allSettled(
			nonDefault.map(async (branch) => {
				try {
					comparisons[branch.name] = await compareBranchSchemas({
						did: params.did,
						repo: params.repo,
						base: defaultBranch,
						head: branch.name,
					});
				} catch {
					comparisons[branch.name] = null;
				}
			})
		);
	}

	return {
		did: params.did,
		repoName: params.repo,
		repo,
		branches,
		defaultBranch,
		comparisons,
	};
};
