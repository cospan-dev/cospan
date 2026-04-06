import { xrpcQuery } from './client.js';
import type {
	PullView,
	PullCommentView,
	PullListResponse as RawPullListResponse,
	PullCommentListResponse as RawPullCommentListResponse,
} from '$lib/generated/views.js';

export type Pull = PullView;
export type PullComment = PullCommentView;

export interface MergePreview {
	conflictCount: number;
	breakingChangeCount: number;
	lensQuality: number | null;
	autoMergeEligible: boolean;
}

export interface PullListResponse {
	items: Pull[];
	cursor: string | null;
}

export interface PullCommentListResponse {
	items: PullComment[];
	cursor: string | null;
}

export async function listPulls(params: {
	did: string;
	repo: string;
	state?: 'open' | 'closed' | 'merged';
	limit?: number;
	cursor?: string;
}): Promise<PullListResponse> {
	const raw = await xrpcQuery<RawPullListResponse>(
		'dev.cospan.repo.pull.list',
		params
	);
	return { items: raw.pulls ?? [], cursor: raw.cursor ?? null };
}

export function getPull(params: {
	did: string;
	repo: string;
	rkey: string;
}): Promise<Pull> {
	return xrpcQuery<Pull>('dev.cospan.repo.pull.get', params);
}

export async function listPullComments(params: {
	did: string;
	repo: string;
	rkey: string;
	limit?: number;
	cursor?: string;
}): Promise<PullCommentListResponse> {
	const pullUri = `at://${params.did}/dev.cospan.repo.pull/${params.rkey}`;
	const raw = await xrpcQuery<RawPullCommentListResponse>(
		'dev.cospan.repo.pull.comment.list',
		{ pull: pullUri, limit: params.limit, cursor: params.cursor }
	);
	return { items: raw.comments ?? [], cursor: raw.cursor ?? null };
}
