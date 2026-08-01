import { clsx, type ClassValue } from 'clsx';
import { extendTailwindMerge } from 'tailwind-merge';

/**
 * The type ramp, restated for tailwind-merge.
 *
 * `twMerge` resolves conflicting utilities by knowing which ones set the same property, and it only
 * knows Tailwind's built-in scale. Our steps (`text-body`, `text-lead`, …) are theme additions, so
 * out of the box it treated `text-xs text-body` as two unrelated classes and emitted both — then
 * whichever rule sat later in the generated stylesheet won, regardless of the order they were
 * written in.
 *
 * That is not hypothetical: the vendored shadcn `Table.Root` ships `text-xs`, our `DataTable`
 * composed `text-body` on top, and every table on the site silently rendered its body copy at 12px
 * instead of 14px. Teaching the merger the ramp makes the later class win the way the call site
 * plainly intends, and fixes the whole class of bug rather than this one instance.
 */
const TEXT_STEPS = [
	'micro',
	'caption',
	'meta',
	'body',
	'lead',
	'heading',
	'prose',
	'metric',
	'title',
] as const;

const twMerge = extendTailwindMerge({
	extend: {
		classGroups: {
			'font-size': [{ text: [...TEXT_STEPS] }],
		},
	},
});

export function cn(...inputs: ClassValue[]) {
	return twMerge(clsx(inputs));
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type WithoutChild<T> = T extends { child?: any } ? Omit<T, 'child'> : T;
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type WithoutChildren<T> = T extends { children?: any } ? Omit<T, 'children'> : T;
export type WithoutChildrenOrChild<T> = WithoutChildren<WithoutChild<T>>;
export type WithElementRef<T, U extends HTMLElement = HTMLElement> = T & { ref?: U | null };
