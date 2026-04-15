/**
 * VCS operations: commit walking, diffs, graph data.
 *
 * These call the appview's node-proxy endpoints which in turn forward
 * to the cospan-node hosting the repo. The frontend never needs to
 * know node URLs.
 */

import { xrpcQuery } from './client.js';

export interface CommitAuthor {
	name: string;
	email: string;
}

export interface Commit {
	oid: string;
	parents: string[];
	summary: string;
	message: string;
	author: CommitAuthor;
	committer: CommitAuthor;
	timestamp: number;
	treeOid: string;
}

export interface ListCommitsResponse {
	commits: Commit[];
	count: number;
	start: string;
}

export interface DiffLine {
	origin: ' ' | '+' | '-' | string;
	content: string;
	oldLineno: number | null;
	newLineno: number | null;
}

export interface DiffHunk {
	oldStart: number;
	oldLines: number;
	newStart: number;
	newLines: number;
	header: string;
	lines: DiffLine[];
}

export type DiffStatus =
	| 'added'
	| 'removed'
	| 'modified'
	| 'renamed'
	| 'copied'
	| 'typechange';

export interface DiffFile {
	path: string;
	oldPath: string | null;
	status: DiffStatus;
	oldOid: string;
	newOid: string;
	additions: number;
	deletions: number;
	binary: boolean;
	hunks: DiffHunk[];
}

export interface DiffCommitsResponse {
	from: string;
	to: string;
	files: DiffFile[];
	totalAdditions: number;
	totalDeletions: number;
	fileCount: number;
}

export function listCommits(params: {
	did: string;
	repo: string;
	ref?: string;
	limit?: number;
}): Promise<ListCommitsResponse> {
	const query: Record<string, string | number | undefined> = {
		did: params.did,
		repo: params.repo,
	};
	if (params.ref) query.ref = params.ref;
	if (params.limit) query.limit = params.limit;
	return xrpcQuery<ListCommitsResponse>('dev.panproto.node.proxy.listCommits', query);
}

export function diffCommits(params: {
	did: string;
	repo: string;
	from: string;
	to: string;
	contextLines?: number;
}): Promise<DiffCommitsResponse> {
	const query: Record<string, string | number | undefined> = {
		did: params.did,
		repo: params.repo,
		from: params.from,
		to: params.to,
	};
	if (params.contextLines !== undefined) query.contextLines = params.contextLines;
	return xrpcQuery<DiffCommitsResponse>('dev.panproto.node.proxy.diffCommits', query);
}

// ─── Commit-graph lane assignment ──────────────────────────────────
//
// Given a list of commits in topological+time order (newest first),
// assign each commit to a lane such that parent-child edges don't
// cross. This is the standard git-log --graph algorithm:
//
// 1. Maintain a sparse array `lanes` where each index is a lane
//    position and the value is the OID of the commit whose child
//    we're waiting to see. Start empty.
// 2. For each commit in order:
//    a. Find its lane (by OID) in `lanes`, or the leftmost empty
//       slot if none exists.
//    b. Place the commit there.
//    c. Remove it from `lanes`, then push its parents into the
//       first free slots (preferring the commit's own lane for the
//       first parent to keep chains straight).
// 3. Collect (fromLane, toLane) edges for each parent link.

export interface GraphNode {
	commit: Commit;
	lane: number;
	row: number;
}

export interface GraphEdge {
	fromRow: number;
	toRow: number;
	fromLane: number;
	toLane: number;
}

export interface CommitGraphLayout {
	nodes: GraphNode[];
	edges: GraphEdge[];
	laneCount: number;
}

export function layoutCommitGraph(commits: Commit[]): CommitGraphLayout {
	const nodes: GraphNode[] = [];
	const edges: GraphEdge[] = [];
	// `lanes[i]` holds the OID of a commit whose child has been placed
	// and is "waiting" for its parent to appear at lane i. A null slot
	// is free.
	let lanes: (string | null)[] = [];
	// row in the graph for each OID (so we can build edges).
	const oidRow = new Map<string, number>();
	const oidLane = new Map<string, number>();

	for (let row = 0; row < commits.length; row++) {
		const commit = commits[row];
		const oid = commit.oid;

		// Step a: find this commit's lane.
		let myLane = lanes.findIndex((l) => l === oid);
		if (myLane === -1) {
			// Not waited-for (e.g. a tip). Place in the leftmost free slot,
			// or append.
			myLane = lanes.findIndex((l) => l === null);
			if (myLane === -1) {
				myLane = lanes.length;
				lanes.push(null);
			}
		}

		nodes.push({ commit, lane: myLane, row });
		oidRow.set(oid, row);
		oidLane.set(oid, myLane);

		// Step b: remove this commit from the lane and push its parents.
		lanes[myLane] = null;

		// For each parent, add it to the lanes list, preferring this
		// commit's own lane for the first parent.
		commit.parents.forEach((parent, idx) => {
			if (idx === 0) {
				// First parent inherits this lane.
				lanes[myLane] = parent;
			} else {
				// Additional parents (merge commits) go to the first free
				// slot, or append.
				let freeIdx = lanes.findIndex((l) => l === null);
				if (freeIdx === -1) {
					freeIdx = lanes.length;
					lanes.push(null);
				}
				lanes[freeIdx] = parent;
			}
		});

		// Compact trailing null slots.
		while (lanes.length > 0 && lanes[lanes.length - 1] === null) {
			lanes.pop();
		}
	}

	// Second pass: build edges.
	// A parent-to-child edge runs from the parent commit's row/lane
	// (if it's in `nodes`) up to the child's row/lane.
	for (const node of nodes) {
		for (const parentOid of node.commit.parents) {
			const parentRow = oidRow.get(parentOid);
			if (parentRow === undefined) continue; // parent is off-screen
			const parentLane = oidLane.get(parentOid);
			if (parentLane === undefined) continue;
			edges.push({
				fromRow: node.row,
				toRow: parentRow,
				fromLane: node.lane,
				toLane: parentLane,
			});
		}
	}

	// Lane count = max lane + 1 across all nodes, or 0.
	const laneCount = nodes.reduce((max, n) => Math.max(max, n.lane + 1), 0);

	return { nodes, edges, laneCount };
}
