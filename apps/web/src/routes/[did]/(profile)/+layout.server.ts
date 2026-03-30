import type { LayoutServerLoad } from './$types';
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

async function fetchFollowCounts(did: string) {
	try {
		const [followersFull, followingFull] = await Promise.all([
			xrpcQuery<{ follows: any[] }>('dev.cospan.graph.follow.list', {
				did,
				direction: 'followers',
				limit: 1000,
			}),
			xrpcQuery<{ follows: any[] }>('dev.cospan.graph.follow.list', {
				did,
				direction: 'following',
				limit: 1000,
			}),
		]);
		return {
			followerCount: followersFull.follows?.length ?? 0,
			followingCount: followingFull.follows?.length ?? 0,
		};
	} catch {
		return { followerCount: 0, followingCount: 0 };
	}
}

export const load: LayoutServerLoad = async ({ params }) => {
	const bskyIdentity = await fetchBlueskyIdentity(params.did);

	let cospanProfile = null;
	try {
		cospanProfile = await getProfile({ did: params.did });
	} catch {}

	const followCounts = await fetchFollowCounts(params.did);

	let repoCount = 0;
	try {
		const repos = await listRepos({ did: params.did, limit: 100 });
		repoCount = repos.items.length;
	} catch {}

	const profile = {
		did: params.did,
		displayName: cospanProfile?.displayName ?? bskyIdentity?.displayName ?? null,
		handle: bskyIdentity?.handle ?? params.did,
		description: cospanProfile?.description ?? bskyIdentity?.description ?? null,
		avatarUrl: bskyIdentity?.avatarUrl ?? null,
		followerCount: followCounts.followerCount,
		followingCount: followCounts.followingCount,
		repoCount,
	};

	return { profile, did: params.did };
};
