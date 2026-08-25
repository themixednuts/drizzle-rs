import type { PageServerLoad } from './$types';
import { repeatabilityPageData } from '#lib/server/bench-data';
import { runServerEffect } from '#lib/server/effect';

export const load: PageServerLoad = ({ platform, url }) =>
	runServerEffect(repeatabilityPageData({ suite: url.searchParams.get('suite') }), platform);
