import type { LayoutServerLoad } from './$types';
import { getRepo } from '$lib/api/repo.js';

export const load: LayoutServerLoad = async ({ params }) => {
	let repo = null;
	try {
		repo = await getRepo({ did: params.did, name: params.repo });
	} catch {}

	return {
		repo,
		did: params.did,
		repoName: params.repo,
	};
};
