import { xrpcQuery } from './client.js';

export interface Repo {
	did: string;
	name: string;
	description: string | null;
	protocol: string;
	starCount: number;
	openIssueCount: number;
	openMrCount: number;
	createdAt: string;
}

export interface RepoListResponse {
	items: Repo[];
	cursor: string | null;
}

interface RawRepoListResponse {
	repos: Repo[];
	cursor: string | null;
}

export async function listRepos(params?: {
	did?: string;
	limit?: number;
	cursor?: string;
}): Promise<RepoListResponse> {
	const raw = await xrpcQuery<RawRepoListResponse>('dev.cospan.repo.list', params);
	return { items: raw.repos ?? [], cursor: raw.cursor ?? null };
}

export function getRepo(params: { did: string; name: string }): Promise<Repo> {
	return xrpcQuery<Repo>('dev.cospan.repo.get', params);
}
