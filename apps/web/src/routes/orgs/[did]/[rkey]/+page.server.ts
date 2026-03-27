import type { PageServerLoad } from './$types';
import { error } from '@sveltejs/kit';
import { getOrg, listOrgMembers } from '$lib/api/org.js';

export const load: PageServerLoad = async ({ params }) => {
	try {
		const [org, members] = await Promise.all([
			getOrg({ did: params.did, rkey: params.rkey }),
			listOrgMembers({ did: params.did, rkey: params.rkey, limit: 50 })
		]);

		return { org, members };
	} catch (err) {
		console.error(`Failed to load org ${params.did}/${params.rkey}:`, err);
		error(404, { message: 'Organization not found' });
	}
};
