import type { PageServerLoad } from './$types';
import { listIssues } from '$lib/api/issue.js';

export const load: PageServerLoad = async ({ params, url }) => {
	const stateParam = url.searchParams.get('state');
	const state = stateParam === 'closed' ? 'closed' : 'open';

	try {
		const issues = await listIssues({
			did: params.did,
			repo: params.repo,
			state,
			limit: 30
		});

		return {
			did: params.did,
			repo: params.repo,
			issues,
			filterState: state
		};
	} catch (err) {
		console.error(`Failed to load issues for ${params.did}/${params.repo}:`, err);
		return {
			did: params.did,
			repo: params.repo,
			issues: { items: [], cursor: null },
			filterState: state
		};
	}
};
