import type { PageServerLoad } from './$types';
import { error } from '@sveltejs/kit';
import { getIssue, getIssueTimeline } from '$lib/api/issue.js';

export const load: PageServerLoad = async ({ params }) => {
	try {
		const [issue, timeline] = await Promise.all([
			getIssue({ did: params.did, repo: params.repo, rkey: params.rkey }),
			getIssueTimeline({ did: params.did, repo: params.repo, rkey: params.rkey, limit: 50 })
		]);

		return {
			did: params.did,
			repo: params.repo,
			issue,
			timeline
		};
	} catch (err) {
		console.error(
			`Failed to load issue ${params.did}/${params.repo}/issues/${params.rkey}:`,
			err
		);
		error(404, { message: 'Issue not found' });
	}
};
