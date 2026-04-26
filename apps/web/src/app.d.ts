// See https://svelte.dev/docs/kit/types#app.d.ts

declare global {
	namespace App {
		interface Locals {
			user?: {
				authenticated: boolean;
				did: string;
				handle: string;
				avatar?: string;
				/** Raw granted scope string from the PDS (space-separated). */
				scope?: string;
			};
		}

		// interface Error {}
		// interface PageData {}
		// interface PageState {}
		// interface Platform {}
	}
}

export {};
