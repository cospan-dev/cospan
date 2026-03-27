/**
 * Format an ISO timestamp as a relative time string (e.g., "3 hours ago").
 * Falls back to an absolute date for anything older than 30 days.
 */
export function timeAgo(iso: string): string {
	const date = new Date(iso);
	const now = Date.now();
	const seconds = Math.floor((now - date.getTime()) / 1000);

	if (seconds < 60) return 'just now';
	if (seconds < 3600) {
		const m = Math.floor(seconds / 60);
		return `${m}m ago`;
	}
	if (seconds < 86400) {
		const h = Math.floor(seconds / 3600);
		return `${h}h ago`;
	}
	if (seconds < 2592000) {
		const d = Math.floor(seconds / 86400);
		return `${d}d ago`;
	}

	return date.toLocaleDateString('en-US', {
		month: 'short',
		day: 'numeric',
		year: 'numeric'
	});
}

/**
 * Format an ISO timestamp as a full date string.
 */
export function formatDate(iso: string): string {
	return new Date(iso).toLocaleDateString('en-US', {
		month: 'short',
		day: 'numeric',
		year: 'numeric'
	});
}
