// ATProto OAuth granular scopes — frontend helpers.
//
// Re-exports and thin wrappers around the codegen output at
// `$lib/generated/scopes.ts`. The generated module is the single source
// of truth: it reads `packages/lexicons/dev/cospan/auth/*.json`,
// transitively expands the `includes` chains, and emits one flat
// `scopes: string[]` array per intent. Run `cargo run -p cospan-codegen`
// to regenerate after editing the permission-set lexicons.

import {
	PERMISSION_SETS,
	buildScopeString as buildScopeStringGenerated,
	type AuthIntent,
	type PermissionSetMeta,
} from '$lib/generated/scopes';

export type { AuthIntent, PermissionSetMeta };
export { PERMISSION_SETS };

const INTENT_ORDER: AuthIntent[] = ['browse', 'contribute', 'maintain', 'own'];

// Each higher intent fully covers every lower one (the lexicons enforce
// this via `includes`). Used by `hasIntent` to gate UI affordances.
const INCLUDES_CHAIN: Record<AuthIntent, AuthIntent[]> = {
	browse: ['browse'],
	contribute: ['browse', 'contribute'],
	maintain: ['browse', 'contribute', 'maintain'],
	own: ['browse', 'contribute', 'maintain', 'own'],
};

/**
 * Build the scope string to request at OAuth login for a given intent.
 *
 * The codegen output already contains the fully expanded inline
 * `repo:`/`rpc:` tokens for each intent, so the frontend never has to
 * hand-mirror the per-intent scope list. The optional `appviewDid` is
 * only used when we someday flip back to `include:` references; today's
 * inline tokens always use `aud=*`.
 */
export function buildScopeString(intent: AuthIntent, _appviewDid: string | null = null): string {
	return buildScopeStringGenerated(intent);
}

/**
 * Return the highest intent tier fully covered by the given granted scope
 * string, or `null` if only `atproto` (or nothing) was granted.
 *
 * Works with both shapes:
 *   - inline scope strings (today): pattern-matches each intent's full
 *     scope set against `granted`,
 *   - `include:` references (future): direct NSID match.
 */
export function grantedIntent(granted: string | null | undefined): AuthIntent | null {
	if (!granted) return null;
	const tokens = new Set(granted.split(/\s+/).filter(Boolean));

	// Fast path: an explicit `include:` reference matched a known intent.
	for (const intent of [...INTENT_ORDER].reverse()) {
		const nsid = PERMISSION_SETS[intent].nsid;
		if (tokens.has(`include:${nsid}`) || tokens.has(`include:${nsid}?aud=*`)) {
			return intent;
		}
	}

	// Inline path: the highest intent whose every scope is in `tokens`.
	for (const intent of [...INTENT_ORDER].reverse()) {
		const required = PERMISSION_SETS[intent].scopes;
		if (required.every((s) => tokens.has(s))) {
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
export const PERMISSION_SET_NSID: Record<AuthIntent, string> = {
	browse: PERMISSION_SETS.browse.nsid,
	contribute: PERMISSION_SETS.contribute.nsid,
	maintain: PERMISSION_SETS.maintain.nsid,
	own: PERMISSION_SETS.own.nsid,
};
