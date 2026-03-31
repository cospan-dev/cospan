import { xrpcQuery } from './client.js';
import type {
	RefUpdateView,
	RefUpdateListResponse as RawRefUpdateListResponse,
} from '$lib/generated/views.js';

export type RefUpdate = RefUpdateView;

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
	const raw = await xrpcQuery<RawRefUpdateListResponse>(
		'dev.cospan.vcs.refUpdate.list',
		params
	);
	return { items: raw.refUpdates ?? [], cursor: raw.cursor ?? null };
}
