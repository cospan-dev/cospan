import type { Actions } from './$types';
import { fail } from '@sveltejs/kit';

export const actions: Actions = {
	createPushToken: async ({ request, cookies }) => {
		const sessionCookie = cookies.get('cospan_session');
		if (!sessionCookie) {
			return fail(401, { error: 'Not signed in. Please sign in first.' });
		}

		// Call the appview directly with the session cookie forwarded.
		const appviewUrl = process.env.APPVIEW_URL ?? 'http://localhost:3000';
		const response = await fetch(`${appviewUrl}/xrpc/dev.cospan.repo.createPushToken`, {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json',
				'Cookie': `cospan_session=${sessionCookie}`,
			},
			body: '{}',
		});

		if (!response.ok) {
			const body = await response.json().catch(() => ({ message: response.statusText }));
			return fail(response.status, { error: body.message ?? `Failed to generate token (${response.status})` });
		}

		const data = await response.json();
		return { token: data.token, did: data.did, expiresIn: data.expiresIn };
	},
};
