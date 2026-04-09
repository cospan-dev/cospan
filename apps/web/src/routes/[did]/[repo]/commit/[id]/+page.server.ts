import type { PageServerLoad } from './$types';
import { listCommits, diffCommits, type Commit, type DiffCommitsResponse } from '$lib/api/vcs.js';

export const load: PageServerLoad = async ({ params }) => {
	let commit: Commit | null = null;
	let diff: DiffCommitsResponse | null = null;
	let error: string | null = null;

	// Fetch the commit + its parent ancestry via listCommits starting at
	// this ref. We only need the commit itself plus its parents for the
	// detail view and first-parent diff.
	try {
		const result = await listCommits({
			did: params.did,
			repo: params.repo,
			ref: params.id,
			limit: 2,
		});
		commit = result.commits[0] ?? null;
	} catch (e) {
		error = (e as Error).message;
	}

	// If we have a commit with at least one parent, fetch the diff
	// against the first parent. For merge commits this is the "mainline"
	// diff; for root commits we skip.
	if (commit && commit.parents.length > 0) {
		try {
			diff = await diffCommits({
				did: params.did,
				repo: params.repo,
				from: commit.parents[0],
				to: commit.oid,
			});
		} catch (e) {
			console.warn('failed to fetch commit diff:', (e as Error).message);
		}
	}

	return {
		did: params.did,
		repoName: params.repo,
		commitId: params.id,
		commit,
		diff,
		error,
	};
};
