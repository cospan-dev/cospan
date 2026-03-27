import { initializeOAuth, logout as oauthLogout } from '$lib/auth/oauth-client';

interface AuthState {
	authenticated: boolean;
	did?: string;
	handle?: string;
	displayName?: string;
	avatar?: string;
	loading: boolean;
}

let state = $state<AuthState>({
	authenticated: false,
	loading: true,
});

let initialized = false;

export async function initAuth(): Promise<void> {
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
		} else {
			state = { authenticated: false, loading: false };
		}
	} catch (e) {
		console.error('Auth init failed:', e);
		state = { authenticated: false, loading: false };
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
