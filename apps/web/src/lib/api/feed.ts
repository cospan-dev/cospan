import { xrpcQuery } from './client.js';

export interface FeedItem {
	type: 'refUpdate' | 'issue' | 'star' | 'pull' | 'follow';
	actorDid: string;
	actorHandle: string | null;
	subjectUri: string | null;
	subjectTitle: string | null;
	repoName: string | null;
	repoDid: string | null;
	summary: string;
	createdAt: string;
}

export interface FeedResponse {
	items: FeedItem[];
	cursor: string | null;
}

export async function getTimeline(params?: {
	limit?: number;
	cursor?: string;
}): Promise<FeedResponse> {
	const raw = await xrpcQuery<{ feed: FeedItem[]; cursor: string | null }>(
		'dev.cospan.feed.getTimeline',
		params
	);
	return { items: raw.feed ?? [], cursor: raw.cursor ?? null };
}
