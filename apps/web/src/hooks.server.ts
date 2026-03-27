import type { Handle } from '@sveltejs/kit';
import { env } from '$env/dynamic/private';

const APPVIEW_URL = env.APPVIEW_URL ?? 'http://localhost:3000';

export const handle: Handle = async ({ event, resolve }) => {
	const sessionCookie = event.cookies.get('cospan_session');
	if (sessionCookie) {
		try {
			const resp = await fetch(`${APPVIEW_URL}/oauth/session`, {
				headers: { Cookie: `cospan_session=${sessionCookie}` }
			});
			if (resp.ok) {
				const session = await resp.json();
				event.locals.user = {
					authenticated: true,
					did: session.did,
					handle: session.handle
				};
			}
		} catch {
			// Session validation failed; proceed as unauthenticated.
		}
	}
	return resolve(event);
};
