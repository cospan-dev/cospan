import type { PageServerLoad } from './$types';
import { searchRepos } from '$lib/api/search.js';

export const load: PageServerLoad = async ({ url }) => {
	const q = url.searchParams.get('q') ?? '';

	if (!q.trim()) {
		return { query: '', results: null };
	}

	try {
		const results = await searchRepos({ q, limit: 30 });
		return { query: q, results };
	} catch (err) {
		console.error(`Failed to search for "${q}":`, err);
		return { query: q, results: { items: [], cursor: null, totalCount: 0 } };
	}
};
