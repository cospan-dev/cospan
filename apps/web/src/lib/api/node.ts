/**
 * Functions to call cospan node XRPC endpoints.
 * Node endpoints are hosted at the node's own URL, not the AppView.
 */

export interface NodeRef {
	name: string;
	target: string;
	type: 'branch' | 'tag';
}

export interface NodeRefListResponse {
	refs: NodeRef[];
}

export interface NodeObject {
	id: string;
	kind: string;
	data: string;
	encoding: string;
	size: number;
}

export interface NodeRepoInfo {
	did: string;
	repo: string;
	defaultBranch: string;
	protocol: string;
	objectCount: number;
}

export interface NodeHead {
	ref: string;
	target: string;
}

async function nodeXrpcQuery<T>(
	nodeUrl: string,
	nsid: string,
	params?: Record<string, string | number | undefined>
): Promise<T> {
	const url = new URL(`/xrpc/${nsid}`, nodeUrl);

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
			// Response body was not JSON; use defaults.
		}

		throw new Error(`Node XRPC error (${response.status}): ${error}: ${message}`);
	}

	return response.json() as Promise<T>;
}

export function listRefs(
	nodeUrl: string,
	did: string,
	repo: string
): Promise<NodeRefListResponse> {
	return nodeXrpcQuery<NodeRefListResponse>(nodeUrl, 'dev.cospan.node.listRefs', {
		did,
		repo
	});
}

export function getObject(
	nodeUrl: string,
	did: string,
	repo: string,
	id: string
): Promise<NodeObject> {
	return nodeXrpcQuery<NodeObject>(nodeUrl, 'dev.cospan.node.getObject', {
		did,
		repo,
		id
	});
}

export function getHead(
	nodeUrl: string,
	did: string,
	repo: string
): Promise<NodeHead> {
	return nodeXrpcQuery<NodeHead>(nodeUrl, 'dev.cospan.node.getHead', {
		did,
		repo
	});
}

export function getRepoInfo(
	nodeUrl: string,
	did: string,
	repo: string
): Promise<NodeRepoInfo> {
	return nodeXrpcQuery<NodeRepoInfo>(nodeUrl, 'dev.cospan.node.getRepoInfo', {
		did,
		repo
	});
}
