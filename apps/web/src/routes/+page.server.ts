import type { PageServerLoad } from './$types';
import { listRepos } from '$lib/api/repo.js';

export const load: PageServerLoad = async ({ url }) => {
	const protocol = url.searchParams.get('protocol') ?? undefined;
	// Landing page shows only Cospan-native repos. Tangled repos are on /import.
	const source = 'cospan';

	try {
		const [trending, recent] = await Promise.all([
			listRepos({ limit: 12, source, protocol } as any),
			listRepos({ limit: 12, source, protocol } as any),
		]);
		return { trending, recent, protocol: protocol ?? null, source: source ?? null };
	} catch {
		const empty = { items: [], cursor: null };
		return { trending: empty, recent: empty, protocol: protocol ?? null, source: source ?? null };
	}
};
