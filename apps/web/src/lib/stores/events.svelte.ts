/**
 * SSE event stream store using Svelte 5 runes.
 * Connects to the /api/events SSE endpoint and tracks latest events by type.
 */

export interface RefUpdateEvent {
	did: string;
	repo: string;
	ref: string;
	newTarget: string;
	commitCount: number;
	timestamp: string;
}

export interface IssueStateEvent {
	did: string;
	repo: string;
	rkey: string;
	state: 'open' | 'closed';
	actorDid: string;
	timestamp: string;
}

export interface PullStateEvent {
	did: string;
	repo: string;
	rkey: string;
	state: 'open' | 'closed' | 'merged';
	actorDid: string;
	timestamp: string;
}

export interface StarEvent {
	did: string;
	repo: string;
	actorDid: string;
	timestamp: string;
}

export interface CommentEvent {
	did: string;
	repo: string;
	rkey: string;
	parentType: 'issue' | 'pull';
	parentRkey: string;
	timestamp: string;
}

type ConnectionState = 'disconnected' | 'connecting' | 'connected' | 'error';

interface EventsState {
	connectionState: ConnectionState;
	latestRefUpdate: RefUpdateEvent | null;
	latestIssueState: IssueStateEvent | null;
	latestPullState: PullStateEvent | null;
	latestStar: StarEvent | null;
	latestComment: CommentEvent | null;
}

let state = $state<EventsState>({
	connectionState: 'disconnected',
	latestRefUpdate: null,
	latestIssueState: null,
	latestPullState: null,
	latestStar: null,
	latestComment: null
});

let eventSource: EventSource | null = null;
let reconnectTimer: ReturnType<typeof setTimeout> | undefined;

export function getEventsState(): EventsState {
	return state;
}

export function connectEvents(repoFilter?: { did: string; repo: string }): void {
	disconnectEvents();

	state.connectionState = 'connecting';

	let url = '/api/events';
	if (repoFilter) {
		const params = new URLSearchParams({
			did: repoFilter.did,
			repo: repoFilter.repo
		});
		url += `?${params.toString()}`;
	}

	eventSource = new EventSource(url);

	eventSource.onopen = () => {
		state.connectionState = 'connected';
	};

	eventSource.onerror = () => {
		state.connectionState = 'error';
		eventSource?.close();
		eventSource = null;

		// Attempt reconnection after 2 seconds
		reconnectTimer = setTimeout(() => {
			connectEvents(repoFilter);
		}, 2000);
	};

	eventSource.addEventListener('refUpdate', (e: MessageEvent) => {
		try {
			state.latestRefUpdate = JSON.parse(e.data);
		} catch {
			// Ignore malformed events.
		}
	});

	eventSource.addEventListener('issueState', (e: MessageEvent) => {
		try {
			state.latestIssueState = JSON.parse(e.data);
		} catch {
			// Ignore malformed events.
		}
	});

	eventSource.addEventListener('pullState', (e: MessageEvent) => {
		try {
			state.latestPullState = JSON.parse(e.data);
		} catch {
			// Ignore malformed events.
		}
	});

	eventSource.addEventListener('star', (e: MessageEvent) => {
		try {
			state.latestStar = JSON.parse(e.data);
		} catch {
			// Ignore malformed events.
		}
	});

	eventSource.addEventListener('comment', (e: MessageEvent) => {
		try {
			state.latestComment = JSON.parse(e.data);
		} catch {
			// Ignore malformed events.
		}
	});
}

export function disconnectEvents(): void {
	if (reconnectTimer !== undefined) {
		clearTimeout(reconnectTimer);
		reconnectTimer = undefined;
	}
	if (eventSource) {
		eventSource.close();
		eventSource = null;
	}
	state.connectionState = 'disconnected';
}
