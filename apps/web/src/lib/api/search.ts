import { xrpcQuery } from './client.js';
import type { Repo } from './repo.js';

export interface SearchReposResponse {
	items: Repo[];
	cursor: string | null;
	totalCount: number;
}

export function searchRepos(params: {
	q: string;
	limit?: number;
	cursor?: string;
}): Promise<SearchReposResponse> {
	return xrpcQuery<SearchReposResponse>('dev.cospan.repo.search', params);
}
