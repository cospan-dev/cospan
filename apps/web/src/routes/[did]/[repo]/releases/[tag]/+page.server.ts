import type { PageServerLoad } from './$types';
import { getRelease } from '$lib/api/release.js';

export const load: PageServerLoad = async ({ params }) => {
	try {
		const release = await getRelease({ did: params.did, repo: params.repo, tag: params.tag });
		return { release, did: params.did, repo: params.repo };
	} catch {
		return { release: null, did: params.did, repo: params.repo, tag: params.tag };
	}
};
