// DID → handle resolution via public Bluesky API with in-memory cache

const cache = new Map<string, string>();
const pending = new Map<string, Promise<string>>();

export async function resolveHandle(did: string): Promise<string> {
	if (cache.has(did)) return cache.get(did)!;
	if (pending.has(did)) return pending.get(did)!;

	const promise = (async () => {
		try {
			const resp = await fetch(
				`https://public.api.bsky.app/xrpc/app.bsky.actor.getProfile?actor=${encodeURIComponent(did)}`
			);
			if (resp.ok) {
				const data = await resp.json();
				const handle = data.handle ?? did;
				cache.set(did, handle);
				return handle;
			}
		} catch {}
		// Fallback: truncated DID
		const fallback = did.startsWith('did:plc:') ? did.slice(8, 18) + '…' : did;
		cache.set(did, fallback);
		return fallback;
	})();

	pending.set(did, promise);
	const result = await promise;
	pending.delete(did);
	return result;
}
