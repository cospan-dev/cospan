import { xrpcQuery } from './client.js';

export interface Webhook {
	rkey: string;
	repo: string;
	url: string;
	secret: string | null;
	events: WebhookEvent[];
	active: boolean;
	createdAt: string;
	lastDeliveryAt: string | null;
	lastDeliveryStatus: number | null;
}

export type WebhookEvent = 'push' | 'issue' | 'pull' | 'star';

export const WEBHOOK_EVENTS: { value: WebhookEvent; label: string; description: string }[] = [
	{ value: 'push', label: 'Push', description: 'Triggered on ref updates (pushes)' },
	{ value: 'issue', label: 'Issue', description: 'Triggered on issue create, update, close' },
	{ value: 'pull', label: 'Merge Request', description: 'Triggered on MR create, update, merge' },
	{ value: 'star', label: 'Star', description: 'Triggered when someone stars the repo' },
];

export interface WebhookListResponse {
	items: Webhook[];
	cursor: string | null;
}

export async function listWebhooks(params: {
	did: string;
	repo: string;
	limit?: number;
	cursor?: string;
}): Promise<WebhookListResponse> {
	const raw = await xrpcQuery<{ webhooks: Webhook[]; cursor: string | null }>(
		'dev.cospan.repo.webhook.list',
		params
	);
	return { items: raw.webhooks ?? [], cursor: raw.cursor ?? null };
}

export async function createWebhook(params: {
	did: string;
	repo: string;
	url: string;
	secret?: string;
	events: WebhookEvent[];
}): Promise<Webhook> {
	const response = await fetch('/xrpc/dev.cospan.repo.webhook.create', {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(params),
	});

	if (!response.ok) {
		const body = await response.json().catch(() => ({}));
		throw new Error(body.message ?? 'Failed to create webhook');
	}

	return response.json();
}

export async function deleteWebhook(params: {
	did: string;
	repo: string;
	rkey: string;
}): Promise<void> {
	const response = await fetch('/xrpc/dev.cospan.repo.webhook.delete', {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(params),
	});

	if (!response.ok) {
		const body = await response.json().catch(() => ({}));
		throw new Error(body.message ?? 'Failed to delete webhook');
	}
}
