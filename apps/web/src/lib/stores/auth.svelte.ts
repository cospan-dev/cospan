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
	state = { authenticated: false, loading: false };
	initialized = false;
}
