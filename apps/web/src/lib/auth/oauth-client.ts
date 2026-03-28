import {
	BrowserOAuthClient,
	type OAuthSession,
} from '@atproto/oauth-client-browser';
import { Agent } from '@atproto/api';

let oauthClient: BrowserOAuthClient | null = null;
let clientInitPromise: Promise<BrowserOAuthClient> | null = null;
let currentSession: OAuthSession | null = null;
let currentAgent: Agent | null = null;

function getClientId(): string {
	if (typeof window === 'undefined') return 'http://localhost';
	const { hostname, port, pathname } = window.location;

	// Loopback mode for local development
	if (hostname === 'localhost' || hostname === '127.0.0.1' || hostname === '[::1]') {
		const redirectUri = `http://127.0.0.1${port ? `:${port}` : ''}${pathname}`;
		return `http://localhost?redirect_uri=${encodeURIComponent(redirectUri)}`;
	}

	// Production: use client metadata URL (served by appview, proxied through SvelteKit)
	return `${window.location.origin}/oauth/client-metadata.json`;
}

export async function getOAuthClient(): Promise<BrowserOAuthClient> {
	if (oauthClient) return oauthClient;
	if (clientInitPromise) return clientInitPromise;

	clientInitPromise = BrowserOAuthClient.load({
		clientId: getClientId(),
		handleResolver: 'https://bsky.social',
	}).then((client) => {
		oauthClient = client;
		return client;
	});

	return clientInitPromise;
}

export async function login(handle: string): Promise<void> {
	const client = await getOAuthClient();
	const url = await client.authorize(handle, { scope: 'atproto' });
	window.location.assign(url.toString());
}

/** Fetch profile via public Bluesky API (no auth required). */
async function fetchPublicProfile(did: string): Promise<{ handle: string; displayName?: string; avatar?: string }> {
	try {
		const resp = await fetch(
			`https://public.api.bsky.app/xrpc/app.bsky.actor.getProfile?actor=${encodeURIComponent(did)}`
		);
		if (resp.ok) {
			const data = await resp.json();
			return { handle: data.handle, displayName: data.displayName, avatar: data.avatar };
		}
	} catch {}
	return { handle: did };
}

export async function initializeOAuth(): Promise<{
	did: string;
	handle: string;
	displayName?: string;
	avatar?: string;
} | null> {
	const client = await getOAuthClient();

	// ATProto OAuth returns params in the hash fragment (#) not query string (?)
	const raw = window.location.hash.startsWith('#')
		? window.location.hash.slice(1)
		: window.location.search.slice(1);
	const params = new URLSearchParams(raw);
	if (params.has('code') && params.has('state')) {
		try {
			const result = await client.callback(params);
			if (result?.session) {
				currentSession = result.session;
				currentAgent = new Agent(result.session);

				// Clean URL
				window.history.replaceState({}, '', window.location.pathname);

				const profile = await fetchPublicProfile(result.session.did);
				return {
					did: result.session.did,
					handle: profile.handle,
					displayName: profile.displayName,
					avatar: profile.avatar,
				};
			}
		} catch (e) {
			console.error('OAuth callback error:', e);
		}
	}

	// Try restoring existing session
	try {
		const result = await client.init();
		if (result?.session) {
			currentSession = result.session;
			currentAgent = new Agent(result.session);
			const profile = await fetchPublicProfile(result.session.did);
			return {
				did: result.session.did,
				handle: profile.handle,
				avatar: profile.avatar,
			};
		}
	} catch {}

	return null;
}

export function getAgent(): Agent | null {
	return currentAgent;
}

export async function logout(): Promise<void> {
	if (currentSession) {
		try { await currentSession.signOut?.(); } catch {}
	}
	currentSession = null;
	currentAgent = null;
	oauthClient = null;
	clientInitPromise = null;
}
