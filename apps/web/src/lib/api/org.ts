import { xrpcQuery } from './client.js';

export interface OrgSummary {
	did: string;
	rkey: string;
	name: string;
	description: string | null;
	avatarUrl: string | null;
	memberCount: number;
	repoCount: number;
	createdAt: string;
}

export interface OrgListResponse {
	items: OrgSummary[];
	cursor: string | null;
}

export interface OrgMember {
	did: string;
	handle: string | null;
	displayName: string | null;
	role: 'admin' | 'member';
	joinedAt: string;
}

export interface OrgMemberListResponse {
	items: OrgMember[];
	cursor: string | null;
}

export async function listOrgs(params?: {
	limit?: number;
	cursor?: string;
}): Promise<OrgListResponse> {
	const raw = await xrpcQuery<{ orgs: OrgSummary[]; cursor: string | null }>('dev.cospan.org.list', params);
	return { items: raw.orgs ?? [], cursor: raw.cursor ?? null };
}

export function getOrg(params: { did: string; rkey: string }): Promise<OrgSummary> {
	return xrpcQuery<OrgSummary>('dev.cospan.org.get', params);
}

export async function listOrgMembers(params: {
	did: string;
	rkey: string;
	limit?: number;
	cursor?: string;
}): Promise<OrgMemberListResponse> {
	const raw = await xrpcQuery<{ members: OrgMember[]; cursor: string | null }>('dev.cospan.org.member.list', params);
	return { items: raw.members ?? [], cursor: raw.cursor ?? null };
}
