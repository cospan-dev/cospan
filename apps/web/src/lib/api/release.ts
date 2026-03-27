import { xrpcQuery } from './client.js';

export interface ReleaseArtifact {
	name: string;
	size: number;
	downloadUrl: string;
	contentType: string;
}

export interface Release {
	rkey: string;
	repo: string;
	did: string;
	tag: string;
	title: string;
	body: string | null;
	artifacts: ReleaseArtifact[];
	draft: boolean;
	prerelease: boolean;
	creatorDid: string;
	creatorHandle: string | null;
	createdAt: string;
}

export interface ReleaseListResponse {
	items: Release[];
	cursor: string | null;
}

export async function listReleases(params: {
	did: string;
	repo: string;
	limit?: number;
	cursor?: string;
}): Promise<ReleaseListResponse> {
	const raw = await xrpcQuery<{ releases: Release[]; cursor: string | null }>(
		'dev.cospan.repo.release.list',
		params
	);
	return { items: raw.releases ?? [], cursor: raw.cursor ?? null };
}

export function getRelease(params: {
	did: string;
	repo: string;
	tag: string;
}): Promise<Release> {
	return xrpcQuery<Release>('dev.cospan.repo.release.get', params);
}

export async function createRelease(params: {
	did: string;
	repo: string;
	tag: string;
	title: string;
	body?: string;
	draft?: boolean;
	prerelease?: boolean;
}): Promise<Release> {
	const response = await fetch('/xrpc/dev.cospan.repo.release.create', {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(params),
	});

	if (!response.ok) {
		const body = await response.json().catch(() => ({}));
		throw new Error(body.message ?? 'Failed to create release');
	}

	return response.json();
}
