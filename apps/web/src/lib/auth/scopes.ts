// ATProto OAuth granular scopes — frontend helpers.
//
// Mirrors the Rust `auth::scope` module in the appview. Supports the
// four Cospan intents and checks whether the session's granted scope
// string covers a required operation. Used to gate UI actions before
// hitting the backend, and to compute the `?intent=...` value for
// login-upgrade redirects.

export type AuthIntent = 'browse' | 'contribute' | 'maintain' | 'own';

const PERMISSION_SET: Record<AuthIntent, string> = {
	browse: 'dev.cospan.auth.readerAccess',
	contribute: 'dev.cospan.auth.contributorAccess',
	maintain: 'dev.cospan.auth.maintainerAccess',
	own: 'dev.cospan.auth.ownerAccess',
};

const INTENT_ORDER: AuthIntent[] = ['browse', 'contribute', 'maintain', 'own'];

// Which permission-sets each intent implies (via `includes:` hierarchy).
// Must match apps/web/../lexicons/dev/cospan/auth/*.json.
const INCLUDES_CHAIN: Record<AuthIntent, AuthIntent[]> = {
	browse: ['browse'],
	contribute: ['browse', 'contribute'],
	maintain: ['browse', 'contribute', 'maintain'],
	own: ['browse', 'contribute', 'maintain', 'own'],
};

/** Build the scope string to request at OAuth login for a given intent. */
export function buildScopeString(intent: AuthIntent, appviewDid: string | null): string {
	const aud = appviewDid ? encodeURIComponent(appviewDid) : '*';
	return `atproto include:${PERMISSION_SET[intent]}?aud=${aud}`;
}

/**
 * Return the highest intent tier fully covered by the given granted scope
 * string, or `null` if only `atproto` (or nothing) was granted.
 */
export function grantedIntent(granted: string | null | undefined): AuthIntent | null {
	if (!granted) return null;
	const tokens = granted.split(/\s+/).filter(Boolean);
	const includes = new Set<string>();
	for (const tok of tokens) {
		if (tok.startsWith('include:')) {
			const head = tok.slice('include:'.length).split('?')[0];
			includes.add(head);
		}
	}
	for (const intent of [...INTENT_ORDER].reverse()) {
		if (includes.has(PERMISSION_SET[intent])) {
			return intent;
		}
	}
	return null;
}

/** Does the granted scope string include at least `required`'s tier? */
export function hasIntent(
	granted: string | null | undefined,
	required: AuthIntent,
): boolean {
	const g = grantedIntent(granted);
	if (!g) return false;
	// Higher intents include lower ones.
	return INCLUDES_CHAIN[g].includes(required);
}

/** Build a login-upgrade URL that re-triggers OAuth with a higher intent. */
export function buildUpgradeUrl(
	appviewUrl: string,
	handle: string,
	intent: AuthIntent,
	returnTo?: string,
): string {
	const params = new URLSearchParams({ handle, intent });
	if (returnTo) params.set('return', returnTo);
	return `${appviewUrl}/oauth/login?${params.toString()}`;
}

export const ALL_INTENTS = INTENT_ORDER;
export const PERMISSION_SET_NSID = PERMISSION_SET;
