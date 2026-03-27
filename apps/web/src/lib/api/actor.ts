import { xrpcQuery } from './client.js';

export interface ActorProfile {
	did: string;
	handle: string;
	displayName: string | null;
	description: string | null;
	avatarUrl: string | null;
	followerCount: number;
	followingCount: number;
	repoCount: number;
}

export function getProfile(params: { did: string }): Promise<ActorProfile> {
	return xrpcQuery<ActorProfile>('dev.cospan.actor.getProfile', params);
}
