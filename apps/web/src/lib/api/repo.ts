import { xrpcQuery } from './client.js';
import type { RepoView, RepoListResponse as RawRepoListResponse } from '$lib/generated/views.js';

export type Repo = RepoView;

export interface RepoListResponse {
	items: Repo[];
	cursor: string | null;
}

export async function listRepos(params?: {
	did?: string;
	source?: string;
	sort?: string;
	query?: string;
	limit?: number;
	cursor?: string;
}): Promise<RepoListResponse> {
	const raw = await xrpcQuery<RawRepoListResponse>('dev.cospan.repo.list', params);
	return { items: raw.repos ?? [], cursor: raw.cursor ?? null };
}

export function getRepo(params: { did: string; name: string }): Promise<Repo> {
	return xrpcQuery<Repo>('dev.cospan.repo.get', params);
}
