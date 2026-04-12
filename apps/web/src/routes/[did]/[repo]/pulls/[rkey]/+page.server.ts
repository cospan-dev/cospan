import type { PageServerLoad } from './$types';
import { error } from '@sveltejs/kit';
import { getPull, listPullComments } from '$lib/api/pull.js';
import {
	compareBranchSchemas,
	type BranchComparisonResponse,
} from '$lib/api/schema.js';
import { listDependents, type DependencyEntry } from '$lib/api/search.js';

export const load: PageServerLoad = async ({ params }) => {
	try {
		const [pull, comments] = await Promise.all([
			getPull({ did: params.did, repo: params.repo, rkey: params.rkey }),
			listPullComments({ did: params.did, repo: params.repo, rkey: params.rkey, limit: 50 })
		]);

		// Structural comparison between source and target branches
		let branchComparison: BranchComparisonResponse | null = null;
		let dependents: DependencyEntry[] = [];

		if (pull.sourceRef && pull.targetRef) {
			// Fetch comparison and dependents in parallel
			const results = await Promise.allSettled([
				compareBranchSchemas({
					did: params.did,
					repo: params.repo,
					base: pull.targetRef.replace('refs/heads/', ''),
					head: pull.sourceRef.replace('refs/heads/', ''),
				}),
				listDependents({
					did: params.did,
					repo: params.repo,
					limit: 20,
				}),
			]);

			if (results[0].status === 'fulfilled') {
				branchComparison = results[0].value;
			}
			if (results[1].status === 'fulfilled') {
				dependents = results[1].value.dependencies;
			}
		}

		return {
			did: params.did,
			repo: params.repo,
			pull,
			comments,
			branchComparison,
			dependents,
		};
	} catch (err) {
		console.error(`Failed to load pull ${params.did}/${params.repo}/pulls/${params.rkey}:`, err);
		error(404, { message: 'Pull request not found' });
	}
};
