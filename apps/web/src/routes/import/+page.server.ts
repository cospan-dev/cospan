import type { PageServerLoad } from './$types';
import { listRepos } from '$lib/api/repo.js';

export const load: PageServerLoad = async ({ url, locals }) => {
	const cursor = url.searchParams.get('cursor') ?? undefined;
	const userDid = (locals as any).user?.did;

	try {
		const repos = await listRepos({ limit: 30, source: 'tangled', sort: 'popular', cursor });
		return { repos, userDid: userDid ?? null };
	} catch {
		return { repos: { items: [], cursor: null }, userDid: userDid ?? null };
	}
};
