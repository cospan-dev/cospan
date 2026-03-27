import { getRepo } from '$lib/api/repo.js';

export const load = async ({ params }: { params: { did: string; repo: string } }) => {
	let protocol: string | null = null;

	try {
		const repo = await getRepo({ did: params.did, name: params.repo });
		protocol = repo.protocol;
	} catch {
		// Non-critical; we can still show the fork page without protocol info.
	}

	return {
		did: params.did,
		repoName: params.repo,
		protocol,
	};
};
