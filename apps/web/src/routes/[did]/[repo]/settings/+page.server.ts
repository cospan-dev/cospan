import { getRepo } from '$lib/api/repo.js';

export const load = async ({ params }: { params: { did: string; repo: string } }) => {
	let repo = null;

	try {
		repo = await getRepo({ did: params.did, name: params.repo });
	} catch {
		// Repo metadata unavailable.
	}

	return {
		did: params.did,
		repoName: params.repo,
		repo,
	};
};
