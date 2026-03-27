import { xrpcQuery } from './client.js';

export type KeyType = 'ssh' | 'gpg';

export interface Key {
	rkey: string;
	type: KeyType;
	title: string;
	fingerprint: string;
	publicKey: string;
	createdAt: string;
}

export interface KeyListResponse {
	items: Key[];
	cursor: string | null;
}

export async function listKeys(params: {
	did: string;
	type?: KeyType;
	limit?: number;
	cursor?: string;
}): Promise<KeyListResponse> {
	const raw = await xrpcQuery<{ keys: Key[]; cursor: string | null }>(
		'dev.cospan.actor.key.list',
		params
	);
	return { items: raw.keys ?? [], cursor: raw.cursor ?? null };
}

export async function addKey(params: {
	did: string;
	type: KeyType;
	title: string;
	publicKey: string;
}): Promise<Key> {
	const response = await fetch('/xrpc/dev.cospan.actor.key.create', {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(params),
	});

	if (!response.ok) {
		const body = await response.json().catch(() => ({}));
		throw new Error(body.message ?? 'Failed to add key');
	}

	return response.json();
}

export async function deleteKey(params: {
	did: string;
	rkey: string;
}): Promise<void> {
	const response = await fetch('/xrpc/dev.cospan.actor.key.delete', {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(params),
	});

	if (!response.ok) {
		const body = await response.json().catch(() => ({}));
		throw new Error(body.message ?? 'Failed to delete key');
	}
}
