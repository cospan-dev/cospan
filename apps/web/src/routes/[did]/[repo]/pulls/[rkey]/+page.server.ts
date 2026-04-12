import type { PageServerLoad } from './$types';
import { error } from '@sveltejs/kit';
import { getPull, listPullComments } from '$lib/api/pull.js';
import {
	compareBranchSchemas,
	type BranchComparisonResponse,
} from '$lib/api/schema.js';

export const load: PageServerLoad = async ({ params }) => {
	try {
		const [pull, comments] = await Promise.all([
			getPull({ did: params.did, repo: params.repo, rkey: params.rkey }),
			listPullComments({ did: params.did, repo: params.repo, rkey: params.rkey, limit: 50 })
		]);

		// Structural comparison between source and target branches
		let branchComparison: BranchComparisonResponse | null = null;
		if (pull.sourceRef && pull.targetRef) {
			try {
				branchComparison = await compareBranchSchemas({
					did: params.did,
					repo: params.repo,
					base: pull.targetRef.replace('refs/heads/', ''),
					head: pull.sourceRef.replace('refs/heads/', ''),
				});
			} catch {
				// Node unreachable; badge won't appear
			}
		}

		return {
			did: params.did,
			repo: params.repo,
			pull,
			comments,
			branchComparison,
		};
	} catch (err) {
		console.error(`Failed to load pull ${params.did}/${params.repo}/pulls/${params.rkey}:`, err);
		error(404, { message: 'Pull request not found' });
	}
};
