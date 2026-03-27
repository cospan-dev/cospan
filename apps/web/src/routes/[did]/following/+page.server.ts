import type { PageServerLoad } from './$types';
import { getProfile } from '$lib/api/actor.js';
import { listFollows } from '$lib/api/social.js';
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
				avatar: data.avatar ?? null,
				followerCount: 0,
				followingCount: 0,
				repoCount: 0,
			};
		}
	} catch {}
	return null;
}

export const load: PageServerLoad = async ({ params }) => {
	let profile = null;
	try {
		profile = await getProfile({ did: params.did });
	} catch {}

	if (!profile) {
		profile = await fetchBlueskyProfile(params.did);
	}

	let following = { items: [] as any[], cursor: null as string | null, totalCount: 0 };
	try {
		following = await listFollows({ did: params.did, direction: 'following', limit: 30 });
	} catch {}

	let repos = { items: [] as any[], cursor: null as string | null };
	try {
		repos = await listRepos({ did: params.did, limit: 30 });
	} catch {}

	return { profile, following, repos, did: params.did };
};
