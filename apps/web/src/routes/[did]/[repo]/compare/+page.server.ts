import { env } from '$env/dynamic/private';
import { getRepo } from '$lib/api/repo.js';
import { listRefs } from '$lib/api/node.js';
import type { NodeRef } from '$lib/api/node.js';
import { diffCommits, type DiffCommitsResponse } from '$lib/api/vcs.js';

const DEFAULT_NODE_URL = env.NODE_URL ?? 'http://localhost:3002';

export const load = async ({ params, url }: { params: { did: string; repo: string }; url: URL }) => {
	const baseRef = url.searchParams.get('base') ?? '';
	const headRef = url.searchParams.get('head') ?? '';

	let repo = null;
	let refs: NodeRef[] = [];

	try {
		repo = await getRepo({ did: params.did, name: params.repo });
	} catch {
		// Repo metadata unavailable.
	}

	try {
		const result = await listRefs(DEFAULT_NODE_URL, params.did, params.repo);
		refs = result.refs;
	} catch {
		// Node might be unreachable.
	}

	// If both refs are selected, resolve their targets and fetch the
	// structural diff through the node proxy. The compare page's Svelte
	// template used to say "structural diff viewer will be displayed
	// here once..." — this is that.
	let diff: DiffCommitsResponse | null = null;
	let diffError: string | null = null;
	const baseTarget = refs.find((r) => r.name === baseRef)?.target ?? null;
	const headTarget = refs.find((r) => r.name === headRef)?.target ?? null;

	if (baseTarget && headTarget && baseTarget !== headTarget) {
		try {
			diff = await diffCommits({
				did: params.did,
				repo: params.repo,
				from: baseTarget,
				to: headTarget,
			});
		} catch (e) {
			diffError = (e as Error).message;
		}
	}

	return {
		did: params.did,
		repoName: params.repo,
		repo,
		refs,
		baseRef,
		headRef,
		diff,
		diffError,
	};
};
