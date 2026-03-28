import type { PageServerLoad } from './$types';
import { getProfile } from '$lib/api/actor.js';
import { listRepos } from '$lib/api/repo.js';

async function fetchBlueskyProfile(did: string) {
	try {
		const resp = await fetch(
			`https://public.api.bsky.app/xrpc/app.bsky.actor.getProfile?actor=${encodeURIComponent(did)}`
		);
		if (resp.ok) {
			const data = await resp.json();
			return {
				did,
				displayName: data.displayName ?? null,
				handle: data.handle ?? did,
				description: data.description ?? null,
				avatarUrl: data.avatar ?? null,
				followerCount: data.followersCount ?? 0,
				followingCount: data.followsCount ?? 0,
				repoCount: 0,
			};
		}
	} catch {}
	return null;
}

export const load: PageServerLoad = async ({ params }) => {
	// Always fetch Bluesky profile for avatar and social stats
	const bskyProfile = await fetchBlueskyProfile(params.did);

	// Try Cospan profile for any Cospan-specific fields
	let cospanProfile = null;
	try {
		cospanProfile = await getProfile({ did: params.did });
	} catch {}

	// Merge: Bluesky provides avatar/stats, Cospan overrides if it has data
	const profile = bskyProfile
		? {
				...bskyProfile,
				...(cospanProfile?.displayName ? { displayName: cospanProfile.displayName } : {}),
				...(cospanProfile?.description ? { description: cospanProfile.description } : {}),
			}
		: cospanProfile;

	let repos = { items: [] as any[], cursor: null as string | null };
	try {
		repos = await listRepos({ did: params.did, limit: 30 });
	} catch {}

	return { profile, repos, did: params.did };
};
