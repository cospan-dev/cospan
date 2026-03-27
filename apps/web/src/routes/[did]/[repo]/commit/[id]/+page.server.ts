import type { PageServerLoad } from './$types';
import { env } from '$env/dynamic/private';
import { getObject } from '$lib/api/node.js';

const DEFAULT_NODE_URL = env.NODE_URL ?? 'http://localhost:3002';

export interface CommitObject {
	id: string;
	kind: string;
	author: string | null;
	committer: string | null;
	message: string | null;
	parents: string[];
	schemaId: string | null;
	migrationId: string | null;
	timestamp: string | null;
	raw: string;
}

function parseCommitObject(obj: { id: string; kind: string; data: string }): CommitObject {
	let parsed: Record<string, unknown> = {};
	try {
		parsed = JSON.parse(obj.data);
	} catch {
		// Data might not be JSON; treat as raw.
	}

	return {
		id: obj.id,
		kind: obj.kind,
		author: (parsed.author as string) ?? null,
		committer: (parsed.committer as string) ?? null,
		message: (parsed.message as string) ?? null,
		parents: Array.isArray(parsed.parents) ? (parsed.parents as string[]) : [],
		schemaId: (parsed.schemaId as string) ?? (parsed.schema_id as string) ?? null,
		migrationId: (parsed.migrationId as string) ?? (parsed.migration_id as string) ?? null,
		timestamp: (parsed.timestamp as string) ?? (parsed.createdAt as string) ?? null,
		raw: obj.data,
	};
}

export const load: PageServerLoad = async ({ params }) => {
	let commit: CommitObject | null = null;

	try {
		const obj = await getObject(DEFAULT_NODE_URL, params.did, params.repo, params.id);
		commit = parseCommitObject(obj);
	} catch (err) {
		console.error(`Failed to load commit ${params.id}:`, err);
	}

	return {
		did: params.did,
		repoName: params.repo,
		commitId: params.id,
		commit,
	};
};
