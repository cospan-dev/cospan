/**
 * Notification API client.
 * Currently a stub that returns empty results. Will be connected
 * to the AppView once the notification indexing pipeline is live.
 */

export interface Notification {
	rkey: string;
	type: 'star' | 'follow' | 'issue' | 'pull' | 'comment' | 'mention' | 'refUpdate';
	reason: string;
	actorDid: string;
	actorHandle: string | null;
	subjectUri: string | null;
	subjectTitle: string | null;
	isRead: boolean;
	createdAt: string;
}

export interface NotificationListResponse {
	items: Notification[];
	cursor: string | null;
	unreadCount: number;
}

export async function listNotifications(_params?: {
	limit?: number;
	cursor?: string;
}): Promise<NotificationListResponse> {
	// Stub: return empty list until notification indexing is implemented
	return { items: [], cursor: null, unreadCount: 0 };
}

export async function getUnreadCount(): Promise<number> {
	// Stub: return 0 until notification indexing is implemented
	return 0;
}

export async function markAllRead(): Promise<void> {
	// Stub: no-op until notification indexing is implemented
}
