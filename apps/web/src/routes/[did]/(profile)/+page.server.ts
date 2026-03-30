import type { PageServerLoad } from './$types';
import { listRepos } from '$lib/api/repo.js';

export const load: PageServerLoad = async ({ params }) => {
	let repos = { items: [] as any[], cursor: null as string | null };
	try {
		repos = await listRepos({ did: params.did, limit: 30 });
	} catch {}

	return { repos };
};
