import { xrpcQuery } from './client.js';
import type {
	OrgView,
	OrgMemberView,
	OrgListResponse as RawOrgListResponse,
	OrgMemberListResponse as RawOrgMemberListResponse,
} from '$lib/generated/views.js';

// The list endpoint returns enriched orgs with counts and avatarUrl.
export interface OrgSummary extends OrgView {
	avatarUrl: string | null;
	memberCount: number;
	repoCount: number;
}

export interface OrgListResponse {
	items: OrgSummary[];
	cursor: string | null;
}

// The list endpoint returns enriched members with handle and displayName.
export interface OrgMember extends OrgMemberView {
	handle: string | null;
	displayName: string | null;
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
	const raw = await xrpcQuery<RawOrgListResponse & { orgs: OrgSummary[] }>('dev.cospan.org.list', params);
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
	const raw = await xrpcQuery<RawOrgMemberListResponse & { members: OrgMember[] }>('dev.cospan.org.member.list', params);
	return { items: raw.members ?? [], cursor: raw.cursor ?? null };
}
