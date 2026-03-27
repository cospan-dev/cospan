import type { PageServerLoad } from './$types';
import { listReleases } from '$lib/api/release.js';

export const load: PageServerLoad = async ({ params }) => {
	try {
		const releases = await listReleases({ did: params.did, repo: params.repo, limit: 25 });
		return { releases, did: params.did, repo: params.repo };
	} catch {
		return { releases: { items: [], cursor: null }, did: params.did, repo: params.repo };
	}
};
