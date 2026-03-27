import { xrpcQuery } from './client.js';

export interface Pull {
	rkey: string;
	repo: string;
	did: string;
	title: string;
	body: string | null;
	state: 'open' | 'closed' | 'merged';
	sourceRef: string;
	targetRef: string;
	sourceRepo: string | null;
	commentCount: number;
	creatorDid: string;
	creatorHandle: string | null;
	mergePreview: MergePreview | null;
	createdAt: string;
	updatedAt: string;
}

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

export interface PullComment {
	rkey: string;
	pull: string;
	body: string;
	reviewDecision: string | null;
	creatorDid: string;
	creatorHandle: string | null;
	createdAt: string;
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
	const raw = await xrpcQuery<{ pulls: Pull[]; cursor: string | null }>(
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
	const raw = await xrpcQuery<{ comments: PullComment[]; cursor: string | null }>(
		'dev.cospan.repo.pull.comment.list',
		params
	);
	return { items: raw.comments ?? [], cursor: raw.cursor ?? null };
}
