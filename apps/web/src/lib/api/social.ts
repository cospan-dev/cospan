import { xrpcQuery } from './client.js';
import type {
	StarView,
	FollowView,
	StarListResponse as RawStarListResponse,
	FollowListResponse as RawFollowListResponse,
} from '$lib/generated/views.js';

export type Star = StarView;
export type Follow = FollowView;

export interface StarListResponse {
	items: Star[];
	cursor: string | null;
	totalCount: number;
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
	const raw = await xrpcQuery<RawStarListResponse & { totalCount?: number }>(
		'dev.cospan.feed.star.list',
		params
	);
	return { items: raw.stars ?? [], cursor: raw.cursor ?? null, totalCount: (raw as any).totalCount ?? 0 };
}

export async function listFollows(params: {
	did: string;
	direction: 'followers' | 'following';
	limit?: number;
	cursor?: string;
}): Promise<FollowListResponse> {
	const raw = await xrpcQuery<RawFollowListResponse & { totalCount?: number }>(
		'dev.cospan.graph.follow.list',
		params
	);
	return {
		items: raw.follows ?? (raw as any).items ?? [],
		cursor: raw.cursor ?? null,
		totalCount: (raw as any).totalCount ?? 0
	};
}
