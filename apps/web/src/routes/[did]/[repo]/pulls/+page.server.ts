import type { PageServerLoad } from './$types';
import { listPulls } from '$lib/api/pull.js';

export const load: PageServerLoad = async ({ params, url }) => {
	const stateParam = url.searchParams.get('state');
	const state =
		stateParam === 'closed' ? 'closed' : stateParam === 'merged' ? 'merged' : 'open';

	try {
		const pulls = await listPulls({
			did: params.did,
			repo: params.repo,
			state,
			limit: 30
		});

		return {
			did: params.did,
			repo: params.repo,
			pulls,
			filterState: state
		};
	} catch (err) {
		console.error(`Failed to load pulls for ${params.did}/${params.repo}:`, err);
		return {
			did: params.did,
			repo: params.repo,
			pulls: { items: [], cursor: null },
			filterState: state
		};
	}
};
