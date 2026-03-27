import { xrpcQuery } from './client.js';

export interface Star {
	rkey: string;
	subject: string;
	actorDid: string;
	actorHandle: string | null;
	createdAt: string;
}

export interface StarListResponse {
	items: Star[];
	cursor: string | null;
	totalCount: number;
}

export interface Follow {
	did: string;
	handle: string | null;
	displayName: string | null;
	avatarUrl: string | null;
}

export interface FollowListResponse {
	items: Follow[];
	cursor: string | null;
	totalCount: number;
}

export async function listStars(params?: {
	did?: string;
	subject?: string;
	limit?: number;
	cursor?: string;
}): Promise<StarListResponse> {
	const raw = await xrpcQuery<{ stars: Star[]; cursor: string | null; totalCount: number }>(
		'dev.cospan.feed.star.list',
		params
	);
	return { items: raw.stars ?? [], cursor: raw.cursor ?? null, totalCount: raw.totalCount ?? 0 };
}

export function listFollows(params: {
	did: string;
	direction: 'followers' | 'following';
	limit?: number;
	cursor?: string;
}): Promise<FollowListResponse> {
	return xrpcQuery<FollowListResponse>('dev.cospan.graph.follow.list', params);
}
