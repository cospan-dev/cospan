import { initializeOAuth, logout as oauthLogout } from '$lib/auth/oauth-client';

interface AuthState {
	authenticated: boolean;
	did?: string;
	handle?: string;
	displayName?: string;
	avatar?: string;
	loading: boolean;
}

export interface ServerUser {
	did: string;
	handle?: string;
	displayName?: string;
	avatar?: string;
}

let state = $state<AuthState>({
	authenticated: false,
	loading: true,
});

let initialized = false;

export async function initAuth(serverUser?: ServerUser | null): Promise<void> {
	if (initialized || typeof window === 'undefined') return;
	initialized = true;

	try {
		const result = await initializeOAuth();
		if (result) {
			state = {
				authenticated: true,
				did: result.did,
				handle: result.handle,
				displayName: result.displayName,
				avatar: result.avatar,
				loading: false,
			};
			// Bridge the browser OAuth session to a server-side cookie so
			// server-rendered pages and form actions can see the session.
			bridgeSession(result.did, result.handle, result.avatar).catch(() => {});
		} else if (serverUser) {
			// IndexedDB session lost but server cookie still valid
			state = {
				authenticated: true,
				did: serverUser.did,
				handle: serverUser.handle,
				displayName: serverUser.displayName,
				avatar: serverUser.avatar,
				loading: false,
			};
		} else {
			state = { authenticated: false, loading: false };
		}
	} catch (e) {
		console.error('Auth init failed:', e);
		// Fall back to server session if available
		if (serverUser) {
			state = {
				authenticated: true,
				did: serverUser.did,
				handle: serverUser.handle,
				displayName: serverUser.displayName,
				avatar: serverUser.avatar,
				loading: false,
			};
		} else {
			state = { authenticated: false, loading: false };
		}
	}
}

export function getAuth(): AuthState {
	return state;
}

export async function doLogout(): Promise<void> {
	await oauthLogout();
	// Clear the server-side session cookie too.
	fetch('/oauth/bridge', { method: 'DELETE', credentials: 'include' }).catch(() => {});
	state = { authenticated: false, loading: false };
	initialized = false;
}

/// Bridge the browser OAuth session to a server-side session cookie.
/// Called after every successful browser OAuth init so that form
/// actions and server-side rendering can see the authenticated user.
async function bridgeSession(did: string, handle?: string, avatar?: string): Promise<void> {
	await fetch('/oauth/bridge', {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ did, handle, avatar }),
		credentials: 'include',
	});
	// Store avatar in a non-httpOnly cookie so SSR can read it for
	// rendering the user menu without waiting for browser OAuth to hydrate.
	if (avatar) {
		document.cookie = `cospan_avatar=${encodeURIComponent(avatar)}; path=/; max-age=604800; SameSite=Lax`;
	}
}
