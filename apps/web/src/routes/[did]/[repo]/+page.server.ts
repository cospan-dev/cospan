import type { PageServerLoad } from './$types';
import { getRepo } from '$lib/api/repo.js';
import { listRefUpdates } from '$lib/api/ref-update.js';
import { listCommits, type Commit } from '$lib/api/vcs.js';

export const load: PageServerLoad = async ({ params }) => {
	try {
		const [repo, refUpdates] = await Promise.all([
			getRepo({ did: params.did, name: params.repo }),
			listRefUpdates({ did: params.did, repo: params.repo, limit: 25 })
		]);

		// Try to fetch the full commit graph from the hosting node.
		// This is best-effort: for Tangled-hosted repos or unreachable
		// nodes we fall back to the flat ref_update list and the graph
		// will just be missing.
		let commits: Commit[] = [];
		try {
			if (repo && repo.source !== 'tangled') {
				const resp = await listCommits({ did: params.did, repo: params.repo, limit: 50 });
				commits = resp.commits;
			}
		} catch (e) {
			console.warn(
				`commit graph unavailable for ${params.did}/${params.repo}:`,
				(e as Error).message
			);
		}

		return { repo, refUpdates, commits, did: params.did, repoName: params.repo };
	} catch (err) {
		console.error(`Failed to load repo ${params.did}/${params.repo}:`, err);
		return {
			repo: null,
			refUpdates: { items: [], cursor: null },
			commits: [] as Commit[],
			did: params.did,
			repoName: params.repo
		};
	}
};
