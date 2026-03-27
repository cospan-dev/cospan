import { getTimeline } from '$lib/api/feed.js';
import type { FeedItem } from '$lib/api/feed.js';

export const load = async () => {
	let items: FeedItem[] = [];
	let cursor: string | null = null;

	try {
		const result = await getTimeline({ limit: 50 });
		items = result.items;
		cursor = result.cursor;
	} catch {
		// Timeline unavailable; return empty feed.
	}

	return { items, cursor };
};
