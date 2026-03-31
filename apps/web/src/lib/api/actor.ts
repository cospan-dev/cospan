import { xrpcQuery } from './client.js';
import type { ActorProfileView } from '$lib/generated/views.js';

// The getProfile endpoint returns an enriched view with counts and a resolved handle.
// These fields are not on the base ActorProfileView Row.
export interface ActorProfile extends ActorProfileView {
	handle: string;
	avatarUrl: string | null;
	followerCount: number;
	followingCount: number;
	repoCount: number;
}

export function getProfile(params: { did: string }): Promise<ActorProfile> {
	return xrpcQuery<ActorProfile>('dev.cospan.actor.getProfile', params);
}
