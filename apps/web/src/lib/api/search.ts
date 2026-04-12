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

// ── Structural search ──────────────────────────────────────────────

export interface StructuralSearchResult {
	[key: string]: unknown;
	_repo_did?: string;
	_repo_name?: string;
}

export interface StructuralSearchResponse {
	anchor: string;
	expression: string;
	limit: number;
	results: StructuralSearchResult[];
	total: number;
}

export function searchStructural(params: {
	q: string;
	anchor?: string;
	limit?: number;
}): Promise<StructuralSearchResponse> {
	return xrpcQuery<StructuralSearchResponse>('dev.cospan.search.structural', params);
}

// ── Dependency impact ──────────────────────────────────────────────

export interface DependencyEntry {
	did: string;
	repo: string;
	[key: string]: unknown;
}

export interface DependencyListResponse {
	dependencies: DependencyEntry[];
	cursor: string | null;
}

export function listDependents(params: {
	did: string;
	repo: string;
	limit?: number;
}): Promise<DependencyListResponse> {
	return xrpcQuery<DependencyListResponse>('dev.cospan.repo.dependency.list', {
		did: params.did,
		repo: params.repo,
		direction: 'dependents',
		limit: params.limit,
	});
}
