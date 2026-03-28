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
	// Try Cospan profile first, fall back to Bluesky public API
	let profile = null;
	try {
		profile = await getProfile({ did: params.did });
	} catch {}

	if (!profile) {
		profile = await fetchBlueskyProfile(params.did);
	}

	let repos = { items: [] as any[], cursor: null as string | null };
	try {
		repos = await listRepos({ did: params.did, limit: 30 });
	} catch {}

	return { profile, repos, did: params.did };
};
