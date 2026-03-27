/**
 * Returns a debounced version of the given function.
 * The function will only execute after `delay` milliseconds
 * have passed since the last invocation.
 */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function debounce<T extends (...args: any[]) => void>(
	fn: T,
	delay: number
): (...args: Parameters<T>) => void {
	let timer: ReturnType<typeof setTimeout> | undefined;

	return (...args: Parameters<T>) => {
		if (timer !== undefined) {
			clearTimeout(timer);
		}
		timer = setTimeout(() => {
			fn(...args);
			timer = undefined;
		}, delay);
	};
}
