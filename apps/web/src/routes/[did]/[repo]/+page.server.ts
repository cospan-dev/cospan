import type { PageServerLoad } from './$types';
import { getRepo } from '$lib/api/repo.js';
import { listRefUpdates } from '$lib/api/ref-update.js';

export const load: PageServerLoad = async ({ params }) => {
	try {
		const [repo, refUpdates] = await Promise.all([
			getRepo({ did: params.did, name: params.repo }),
			listRefUpdates({ did: params.did, repo: params.repo, limit: 25 })
		]);

		return { repo, refUpdates, did: params.did, repoName: params.repo };
	} catch (err) {
		console.error(`Failed to load repo ${params.did}/${params.repo}:`, err);
		return {
			repo: null,
			refUpdates: { items: [], cursor: null },
			did: params.did,
			repoName: params.repo
		};
	}
};
