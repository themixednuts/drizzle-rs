import type { LayoutServerLoad } from './$types';
import { runServerEffect } from '#lib/server/effect';
import { BenchStore } from '#lib/server/bench-store';
import { Effect } from 'effect';

/**
 * One fact, shared by every view under `/runs`: is there more than one commit of
 * history in the bucket?
 *
 * The History tab is offered only when the answer is yes. Until a second commit
 * publishes, a trend line is a single point, and a tab that always lands on
 * "select a library · 0 runs" teaches people to stop clicking tabs.
 *
 * This reads the index, which every page under here reads anyway, so it is free:
 * the store memoises within a request.
 */
export const load: LayoutServerLoad = ({ platform }) =>
	runServerEffect(
		Effect.gen(function* () {
			const store = yield* BenchStore;
			const index = yield* store.readIndexOrEmpty;
			const commits = new Set(index.runs.map((run) => run.git));
			return { hasHistory: commits.size > 1 };
		}),
		platform,
	);
