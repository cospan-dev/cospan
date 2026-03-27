import type { PageServerLoad } from './$types';
import { error } from '@sveltejs/kit';
import { getPull, listPullComments } from '$lib/api/pull.js';

export const load: PageServerLoad = async ({ params }) => {
	try {
		const [pull, comments] = await Promise.all([
			getPull({ did: params.did, repo: params.repo, rkey: params.rkey }),
			listPullComments({ did: params.did, repo: params.repo, rkey: params.rkey, limit: 50 })
		]);

		return {
			did: params.did,
			repo: params.repo,
			pull,
			comments
		};
	} catch (err) {
		console.error(`Failed to load pull ${params.did}/${params.repo}/pulls/${params.rkey}:`, err);
		error(404, { message: 'Pull request not found' });
	}
};
