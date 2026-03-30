import type { PageServerLoad } from './$types';
import { getProfile } from '$lib/api/actor.js';
import { listRepos } from '$lib/api/repo.js';
import { xrpcQuery } from '$lib/api/client.js';

async function fetchBlueskyIdentity(did: string) {
	try {
		const resp = await fetch(
			`https://public.api.bsky.app/xrpc/app.bsky.actor.getProfile?actor=${encodeURIComponent(did)}`
		);
		if (resp.ok) {
			const data = await resp.json();
			return {
				displayName: data.displayName ?? null,
				handle: data.handle ?? did,
				description: data.description ?? null,
				avatarUrl: data.avatar ?? null,
			};
		}
	} catch {}
	return null;
}

async function fetchCospanFollowCounts(did: string) {
	try {
		const [followers, following] = await Promise.all([
			xrpcQuery<{ follows: any[] }>('dev.cospan.graph.follow.list', { did, direction: 'followers', limit: 1 }),
			xrpcQuery<{ follows: any[] }>('dev.cospan.graph.follow.list', { did, direction: 'following', limit: 1 }),
		]);
		// The API doesn't return total counts, just paginated lists.
		// For now, fetch a larger batch and count. TODO: add count endpoint.
		const [followersFull, followingFull] = await Promise.all([
			xrpcQuery<{ follows: any[] }>('dev.cospan.graph.follow.list', { did, direction: 'followers', limit: 1000 }),
			xrpcQuery<{ follows: any[] }>('dev.cospan.graph.follow.list', { did, direction: 'following', limit: 1000 }),
		]);
		return {
			followerCount: followersFull.follows?.length ?? 0,
			followingCount: followingFull.follows?.length ?? 0,
		};
	} catch {
		return { followerCount: 0, followingCount: 0 };
	}
}

export const load: PageServerLoad = async ({ params }) => {
	// Fetch identity from Bluesky (avatar, handle, description only)
	const bskyIdentity = await fetchBlueskyIdentity(params.did);

	// Fetch Cospan-specific profile
	let cospanProfile = null;
	try {
		cospanProfile = await getProfile({ did: params.did });
	} catch {}

	// Fetch follow counts from Cospan database (includes Tangled follows)
	const followCounts = await fetchCospanFollowCounts(params.did);

	let repos = { items: [] as any[], cursor: null as string | null };
	try {
		repos = await listRepos({ did: params.did, limit: 30 });
	} catch {}

	const profile = {
		did: params.did,
		displayName: cospanProfile?.displayName ?? bskyIdentity?.displayName ?? null,
		handle: bskyIdentity?.handle ?? params.did,
		description: cospanProfile?.description ?? bskyIdentity?.description ?? null,
		avatarUrl: bskyIdentity?.avatarUrl ?? null,
		followerCount: followCounts.followerCount,
		followingCount: followCounts.followingCount,
		repoCount: repos.items.length,
	};

	return { profile, repos, did: params.did };
};
