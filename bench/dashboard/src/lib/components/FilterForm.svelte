<script lang="ts">
	import { goto } from '$app/navigation';
	import { buttonVariants } from '#lib/components/ui/button/index.js';
	import { cn } from '#lib/utils.js';
	import type { Snippet } from 'svelte';

	/**
	 * A filter as a real `GET` form.
	 *
	 * Filters are URL state, not mutations, so they are deliberately not built on Kit's `form()`
	 * remote function — that is POST-only (`form.js` sets `instance.method = 'POST'`), and a POST
	 * would cost these controls their shareable, cacheable, back-navigable addresses. A native GET
	 * form gives exactly the right semantics with no JavaScript at all: submitting rewrites the
	 * query string, and the server renders the result.
	 *
	 * The enhancement is a client-side navigation instead of a document load. The submit button
	 * stays in the markup and is hidden by the `js:` variant, which keys off a class set by an
	 * inline head script — so it is hidden before first paint rather than after hydration, and
	 * nothing moves.
	 */
	let {
		action,
		children,
		submitLabel = 'apply',
		class: className,
	}: {
		action: string;
		children: Snippet;
		submitLabel?: string;
		class?: string;
	} = $props();

	function submit(event: SubmitEvent & { currentTarget: HTMLFormElement }): void {
		event.preventDefault();

		const params = new URLSearchParams();
		for (const [name, value] of new FormData(event.currentTarget)) {
			if (typeof value === 'string' && value !== '') params.set(name, value);
		}

		const query = params.toString();
		void goto(query ? `${action}?${query}` : action, { reset: false });
	}
</script>

<form method="GET" {action} onsubmit={submit} class={cn('contents', className)}>
	{@render children()}
	<button type="submit" class={cn(buttonVariants({ variant: 'outline', size: 'xs' }), 'js:hidden')}>
		{submitLabel}
	</button>
</form>
