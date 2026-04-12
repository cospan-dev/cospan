import type { PageServerLoad } from './$types';
import { getRepo } from '$lib/api/repo.js';
import { listRefUpdates } from '$lib/api/ref-update.js';
import { listCommits, type Commit } from '$lib/api/vcs.js';
import {
	getProjectSchema,
	getCommitSchemaStats,
	type ProjectSchemaResponse,
	type CommitSchemaStatsResponse,
} from '$lib/api/schema.js';

export const load: PageServerLoad = async ({ params }) => {
	try {
		const [repo, refUpdates] = await Promise.all([
			getRepo({ did: params.did, name: params.repo }),
			listRefUpdates({ did: params.did, repo: params.repo, limit: 25 })
		]);

		let commits: Commit[] = [];
		let projectSchema: ProjectSchemaResponse | null = null;
		let schemaStats: CommitSchemaStatsResponse | null = null;

		if (repo && repo.source !== 'tangled') {
			// Fetch commit graph, project schema, and schema stats in parallel.
			// All are best-effort: if the node is unreachable, the page
			// still renders with what we have.
			const results = await Promise.allSettled([
				listCommits({
					did: params.did,
					repo: params.repo,
					ref: repo?.defaultBranch || 'main',
					limit: 50,
				}),
				getProjectSchema({ did: params.did, repo: params.repo }),
				getCommitSchemaStats({ did: params.did, repo: params.repo, limit: 30 }),
			]);

			if (results[0].status === 'fulfilled') {
				commits = results[0].value.commits;
			}
			if (results[1].status === 'fulfilled') {
				projectSchema = results[1].value;
			}
			if (results[2].status === 'fulfilled') {
				schemaStats = results[2].value;
			}
		}

		return {
			repo, refUpdates, commits,
			projectSchema, schemaStats,
			did: params.did, repoName: params.repo,
		};
	} catch (err) {
		console.error(`Failed to load repo ${params.did}/${params.repo}:`, err);
		return {
			repo: null,
			refUpdates: { items: [], cursor: null },
			commits: [] as Commit[],
			projectSchema: null as ProjectSchemaResponse | null,
			schemaStats: null as CommitSchemaStatsResponse | null,
			did: params.did,
			repoName: params.repo
		};
	}
};
