import type { PageServerLoad } from './$types';
import { listWebhooks } from '$lib/api/webhook.js';

export const load: PageServerLoad = async ({ params }) => {
	try {
		const webhooks = await listWebhooks({ did: params.did, repo: params.repo, limit: 50 });
		return { webhooks, did: params.did, repo: params.repo };
	} catch {
		return { webhooks: { items: [], cursor: null }, did: params.did, repo: params.repo };
	}
};
