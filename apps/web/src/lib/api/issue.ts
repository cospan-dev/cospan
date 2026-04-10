import { xrpcQuery } from './client.js';
import type { IssueView, IssueCommentView, IssueStateView, IssueListResponse as RawIssueListResponse } from '$lib/generated/views.js';

export type Issue = IssueView;
export type IssueComment = IssueCommentView;
export type IssueStateChange = IssueStateView;

export interface IssueListResponse {
	items: Issue[];
	cursor: string | null;
}

// Timeline is a composite type - not directly generated from a single Row
export interface IssueTimelineResponse {
	timeline: Record<string, unknown>[];
	cursor: string | null;
}

export async function listIssues(params: {
	did: string;
	repo: string;
	state?: 'open' | 'closed';
	limit?: number;
	cursor?: string;
}): Promise<IssueListResponse> {
	const raw = await xrpcQuery<RawIssueListResponse>(
		'dev.cospan.repo.issue.list',
		params
	);
	return { items: raw.issues ?? [], cursor: raw.cursor ?? null };
}

export function getIssue(params: {
	did: string;
	repo: string;
	rkey: string;
}): Promise<Issue> {
	return xrpcQuery<Issue>('dev.cospan.repo.issue.get', params);
}

export function getIssueTimeline(params: {
	did: string;
	repo: string;
	rkey: string;
	limit?: number;
	cursor?: string;
}): Promise<IssueTimelineResponse> {
	const issueUri = `at://${params.did}/dev.cospan.repo.issue/${params.rkey}`;
	return xrpcQuery<IssueTimelineResponse>('dev.cospan.repo.issue.getTimeline', {
		issue: issueUri,
		limit: params.limit,
		cursor: params.cursor,
	});
}
