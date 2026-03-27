import type { PageServerLoad } from './$types';
import { listOrgs } from '$lib/api/org.js';

export const load: PageServerLoad = async () => {
	try {
		const orgs = await listOrgs({ limit: 30 });
		return { orgs };
	} catch (err) {
		console.error('Failed to load orgs list:', err);
		return { orgs: { items: [], cursor: null } };
	}
};
