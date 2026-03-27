import type { PageServerLoad } from './$types';
import { listRepos } from '$lib/api/repo.js';

export const load: PageServerLoad = async ({ url }) => {
	const protocol = url.searchParams.get('protocol') ?? undefined;

	try {
		const [trending, recent] = await Promise.all([
			listRepos({ limit: 12, sort: 'stars' as any, protocol } as any),
			listRepos({ limit: 12, sort: 'updated' as any, protocol } as any),
		]);
		return { trending, recent, protocol: protocol ?? null };
	} catch {
		const empty = { items: [], cursor: null };
		return { trending: empty, recent: empty, protocol: protocol ?? null };
	}
};
