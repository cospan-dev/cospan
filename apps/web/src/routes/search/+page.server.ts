import type { PageServerLoad } from './$types';
import { searchRepos, searchStructural } from '$lib/api/search.js';

export const load: PageServerLoad = async ({ url }) => {
	const q = url.searchParams.get('q') ?? '';
	const mode = url.searchParams.get('mode') ?? 'repos';
	const anchor = url.searchParams.get('anchor') ?? 'function';

	if (!q.trim()) {
		return { query: '', mode, anchor, results: null, structuralResults: null };
	}

	try {
		if (mode === 'structural') {
			const structuralResults = await searchStructural({
				q,
				anchor,
				limit: 30,
			});
			return { query: q, mode, anchor, results: null, structuralResults };
		}

		const results = await searchRepos({ q, limit: 30 });
		return { query: q, mode, anchor, results, structuralResults: null };
	} catch (err) {
		console.error(`Failed to search for "${q}":`, err);
		return {
			query: q,
			mode,
			anchor,
			results: mode === 'repos' ? { items: [], cursor: null, totalCount: 0 } : null,
			structuralResults: mode === 'structural' ? { anchor, expression: q, limit: 30, results: [], total: 0 } : null,
		};
	}
};
