import { xrpcQuery } from './client.js';

export interface AlgebraicChecks {
	gatTypeCheck: string | null;
	equationVerification: string | null;
	lensLawCheck: string | null;
	breakingChangeCheck: string | null;
}

export interface RefUpdate {
	rkey: string;
	repo: string;
	ref: string;
	oldTarget: string | null;
	newTarget: string;
	protocol: string;
	commitCount: number;
	migrationId: string | null;
	lensId: string | null;
	lensQuality: number | null;
	breakingChangeCount: number;
	algebraicChecks: AlgebraicChecks | null;
	committerDid: string;
	committerHandle: string | null;
	createdAt: string;
}

export interface RefUpdateListResponse {
	items: RefUpdate[];
	cursor: string | null;
}

export async function listRefUpdates(params: {
	did: string;
	repo: string;
	ref?: string;
	limit?: number;
	cursor?: string;
}): Promise<RefUpdateListResponse> {
	const raw = await xrpcQuery<{ refUpdates: RefUpdate[]; cursor: string | null }>(
		'dev.cospan.vcs.refUpdate.list',
		params
	);
	return { items: raw.refUpdates ?? [], cursor: raw.cursor ?? null };
}
