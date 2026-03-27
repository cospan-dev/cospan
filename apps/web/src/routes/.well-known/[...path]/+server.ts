import type { RequestHandler } from './$types';

const APPVIEW_URL = process.env.APPVIEW_URL ?? 'http://localhost:3000';

export const GET: RequestHandler = async ({ request, params }) => {
	const url = new URL(request.url);
	const target = `${APPVIEW_URL}/.well-known/${params.path}${url.search}`;
	const resp = await fetch(target, {
		headers: { 'Accept': request.headers.get('Accept') ?? 'application/json' },
	});
	return new Response(resp.body, {
		status: resp.status,
		headers: Object.fromEntries(resp.headers.entries()),
	});
};
