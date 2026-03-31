import { browser } from '$app/environment';

function getAppviewUrl(): string {
	if (browser) {
		// Client-side: use the origin (same-host proxy or direct)
		return '';
	}
	// Server-side: use env var or default
	return process.env.APPVIEW_URL ?? 'http://localhost:3000';
}

export class XRPCError extends Error {
	constructor(
		public status: number,
		public error: string,
		message: string
	) {
		super(message);
		this.name = 'XRPCError';
	}
}

export async function xrpcQuery<T>(
	nsid: string,
	params?: Record<string, string | number | undefined>
): Promise<T> {
	const base = getAppviewUrl();
	const url = new URL(`/xrpc/${nsid}`, base || (typeof window !== 'undefined' ? window.location.origin : 'http://localhost:3000'));

	if (params) {
		for (const [key, value] of Object.entries(params)) {
			if (value !== undefined) {
				url.searchParams.set(key, String(value));
			}
		}
	}

	const response = await fetch(url.toString());

	if (!response.ok) {
		let error = 'Unknown';
		let message = response.statusText;

		try {
			const body = await response.json();
			error = body.error ?? error;
			message = body.message ?? message;
		} catch {
			// Response body was not JSON
		}

		throw new XRPCError(response.status, error, message);
	}

	return response.json() as Promise<T>;
}
