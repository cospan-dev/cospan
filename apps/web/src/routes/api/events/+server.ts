import type { RequestHandler } from './$types';

const APPVIEW_URL = import.meta.env.VITE_APPVIEW_URL ?? 'http://localhost:3000';

export const GET: RequestHandler = async ({ url, request }) => {
	const did = url.searchParams.get('did');
	const repo = url.searchParams.get('repo');

	// Build upstream SSE URL
	const upstreamUrl = new URL('/events', APPVIEW_URL);
	if (did) upstreamUrl.searchParams.set('did', did);
	if (repo) upstreamUrl.searchParams.set('repo', repo);

	const stream = new ReadableStream({
		async start(controller) {
			const encoder = new TextEncoder();

			// Send initial keepalive comment
			controller.enqueue(encoder.encode(': connected\n\n'));

			let upstreamController: AbortController | null = new AbortController();

			// Handle client disconnect
			request.signal.addEventListener('abort', () => {
				upstreamController?.abort();
				upstreamController = null;
				try {
					controller.close();
				} catch {
					// Already closed.
				}
			});

			try {
				const response = await fetch(upstreamUrl.toString(), {
					headers: { Accept: 'text/event-stream' },
					signal: upstreamController.signal
				});

				if (!response.ok || !response.body) {
					// If upstream is unavailable, send heartbeats to keep connection alive
					// so the client can attempt reconnection gracefully.
					const heartbeat = setInterval(() => {
						try {
							controller.enqueue(encoder.encode(': heartbeat\n\n'));
						} catch {
							clearInterval(heartbeat);
						}
					}, 15000);

					request.signal.addEventListener('abort', () => {
						clearInterval(heartbeat);
					});

					return;
				}

				const reader = response.body.getReader();
				const decoder = new TextDecoder();

				while (true) {
					const { done, value } = await reader.read();
					if (done) break;

					const text = decoder.decode(value, { stream: true });
					controller.enqueue(encoder.encode(text));
				}
			} catch (e) {
				if (e instanceof DOMException && e.name === 'AbortError') {
					// Client disconnected; this is normal.
					return;
				}

				// Upstream failed; send an error event to the client
				const errorEvent = `event: error\ndata: ${JSON.stringify({ message: 'Upstream connection lost' })}\n\n`;
				try {
					controller.enqueue(encoder.encode(errorEvent));
				} catch {
					// Controller already closed.
				}
			}
		}
	});

	return new Response(stream, {
		headers: {
			'Content-Type': 'text/event-stream',
			'Cache-Control': 'no-cache, no-store, must-revalidate',
			Connection: 'keep-alive',
			'X-Accel-Buffering': 'no'
		}
	});
};
