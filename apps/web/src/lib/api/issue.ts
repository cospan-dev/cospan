import { xrpcQuery } from './client.js';

export interface Issue {
	rkey: string;
	repo: string;
	did: string;
	title: string;
	body: string | null;
	state: 'open' | 'closed';
	labels: string[];
	commentCount: number;
	creatorDid: string;
	creatorHandle: string | null;
	createdAt: string;
	updatedAt: string;
}

export interface IssueListResponse {
	items: Issue[];
	cursor: string | null;
}

export interface IssueComment {
	rkey: string;
	issue: string;
	body: string;
	creatorDid: string;
	creatorHandle: string | null;
	createdAt: string;
}

export interface IssueStateChange {
	rkey: string;
	issue: string;
	state: 'open' | 'closed';
	reason: string | null;
	actorDid: string;
	actorHandle: string | null;
	createdAt: string;
}

export type TimelineEvent =
	| { type: 'comment'; data: IssueComment }
	| { type: 'stateChange'; data: IssueStateChange };

export interface IssueTimelineResponse {
	events: TimelineEvent[];
	cursor: string | null;
}

export async function listIssues(params: {
	did: string;
	repo: string;
	state?: 'open' | 'closed';
	limit?: number;
	cursor?: string;
}): Promise<IssueListResponse> {
	const raw = await xrpcQuery<{ issues: Issue[]; cursor: string | null }>(
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
	return xrpcQuery<IssueTimelineResponse>('dev.cospan.repo.issue.getTimeline', params);
}
