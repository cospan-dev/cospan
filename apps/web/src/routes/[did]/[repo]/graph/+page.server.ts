import type { PageServerLoad } from './$types';
import { getDependencyGraph, type DependencyGraphResponse } from '$lib/api/schema.js';

export const load: PageServerLoad = async ({ params }) => {
	let graph: DependencyGraphResponse | null = null;
	try {
		graph = await getDependencyGraph({
			did: params.did,
			repo: params.repo,
			maxFiles: 200,
		});
	} catch {
		// Node unreachable; graph unavailable
	}

	return {
		did: params.did,
		repoName: params.repo,
		graph,
	};
};
