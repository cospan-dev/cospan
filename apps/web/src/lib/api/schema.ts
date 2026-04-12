/**
 * Schema intelligence API: project-level analysis, file schemas,
 * branch comparisons, commit stats, and dependency graphs.
 *
 * All data originates from panproto's structural parsing engine
 * (248 tree-sitter languages + 50+ protocol parsers) running on
 * the cospan node, proxied through the appview.
 */

import { xrpcQuery } from './client.js';

// ── Types ──────────────────────────────────────────────────────────

export interface SchemaLanguage {
	name: string;
	fileCount: number;
	vertexCount: number;
}

export interface FileSchemaEntry {
	path: string;
	language: string;
	vertexCount: number;
	edgeCount: number;
	topNames: string[];
}

export interface ProjectSchemaResponse {
	commit: string;
	protocol: string;
	totalVertexCount: number;
	totalEdgeCount: number;
	fileCount: number;
	parsedFileCount: number;
	languages: SchemaLanguage[];
	fileSchemas: FileSchemaEntry[];
}

export interface CommitSchemaStat {
	oid: string;
	timestamp: number;
	summary: string;
	totalVertexCount: number;
	totalEdgeCount: number;
	parsedFileCount: number;
	breakingChangeCount: number;
	nonBreakingChangeCount: number;
}

export interface CommitSchemaStatsResponse {
	commits: CommitSchemaStat[];
}

export interface SchemaVertex {
	id: string;
	name: string;
	kind: string;
	humanLabel: string;
}

export interface SchemaEdge {
	src: string;
	tgt: string;
	kind: string;
	name: string | null;
	humanLabel: string;
}

export interface FileSchemaResponse {
	path: string;
	commit: string;
	language: string | null;
	vertexCount: number;
	edgeCount: number;
	vertices: SchemaVertex[];
	edges: SchemaEdge[];
}

export interface StructuralChange {
	kind: string;
	label: string;
	vertexId?: string;
	src?: string;
	tgt?: string;
}

export interface BranchComparisonResponse {
	base: { ref: string; oid: string };
	head: { ref: string; oid: string };
	compatible: boolean;
	verdict: 'compatible' | 'breaking';
	breakingCount: number;
	nonBreakingCount: number;
	addedVertices: number;
	removedVertices: number;
	addedEdges: number;
	removedEdges: number;
	breakingChanges: StructuralChange[];
	nonBreakingChanges: StructuralChange[];
	changedFiles: string[];
	baseVertexCount: number;
	headVertexCount: number;
}

export interface DependencyNode {
	id: string;
	language: string;
	vertexCount: number;
	label: string;
}

export interface DependencyEdge {
	src: string;
	tgt: string;
	kind: string;
	label: string;
}

export interface DependencyGraphResponse {
	commit: string;
	nodes: DependencyNode[];
	edges: DependencyEdge[];
}

// ── API functions ──────────────────────────────────────────────────

export function getProjectSchema(params: {
	did: string;
	repo: string;
	commit?: string;
	maxFiles?: number;
}): Promise<ProjectSchemaResponse> {
	return xrpcQuery<ProjectSchemaResponse>(
		'dev.panproto.node.proxy.getProjectSchema',
		{
			did: params.did,
			repo: params.repo,
			commit: params.commit,
			maxFiles: params.maxFiles,
		}
	);
}

export function getCommitSchemaStats(params: {
	did: string;
	repo: string;
	ref?: string;
	limit?: number;
}): Promise<CommitSchemaStatsResponse> {
	return xrpcQuery<CommitSchemaStatsResponse>(
		'dev.panproto.node.proxy.getCommitSchemaStats',
		{
			did: params.did,
			repo: params.repo,
			ref: params.ref,
			limit: params.limit,
		}
	);
}

export function getFileSchema(params: {
	did: string;
	repo: string;
	commit: string;
	path: string;
}): Promise<FileSchemaResponse> {
	return xrpcQuery<FileSchemaResponse>(
		'dev.panproto.node.proxy.getFileSchema',
		params
	);
}

export function compareBranchSchemas(params: {
	did: string;
	repo: string;
	base: string;
	head: string;
}): Promise<BranchComparisonResponse> {
	return xrpcQuery<BranchComparisonResponse>(
		'dev.panproto.node.proxy.compareBranchSchemas',
		params
	);
}

export function getDependencyGraph(params: {
	did: string;
	repo: string;
	commit?: string;
	maxFiles?: number;
}): Promise<DependencyGraphResponse> {
	return xrpcQuery<DependencyGraphResponse>(
		'dev.panproto.node.proxy.getDependencyGraph',
		{
			did: params.did,
			repo: params.repo,
			commit: params.commit,
			maxFiles: params.maxFiles,
		}
	);
}
