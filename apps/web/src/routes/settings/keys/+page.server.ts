import type { Actions } from './$types';
import { fail } from '@sveltejs/kit';

export const actions: Actions = {
	createPushToken: async ({ request, fetch }) => {
		// Use SvelteKit's fetch which automatically forwards cookies
		const response = await fetch('/xrpc/dev.cospan.repo.createPushToken', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: '{}',
		});

		if (!response.ok) {
			const body = await response.json().catch(() => ({ message: response.statusText }));
			return fail(response.status, { error: body.message ?? 'Failed to generate token' });
		}

		const data = await response.json();
		return { token: data.token, did: data.did, expiresIn: data.expiresIn };
	},
};
