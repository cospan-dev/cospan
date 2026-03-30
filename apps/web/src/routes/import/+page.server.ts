import type { PageServerLoad } from './$types';
import { listRepos } from '$lib/api/repo.js';

export const load: PageServerLoad = async ({ url }) => {
	const protocol = url.searchParams.get('protocol') ?? undefined;

	try {
		const repos = await listRepos({ limit: 50, source: 'tangled', protocol } as any);
		return { repos, protocol: protocol ?? null };
	} catch {
		return { repos: { items: [], cursor: null }, protocol: protocol ?? null };
	}
};
