import './cloudflare.d.ts';
import type { ISRRequestScope } from 'cloudflare-isr';

declare global {
	namespace App {
		interface Locals {
			/**
			 * Populated by the ISR handle in production only. In `vite dev` the handle is bypassed,
			 * so this is genuinely absent and must be typed optional.
			 */
			isr?: ISRRequestScope;
		}
		interface Platform {
			env?: {
				BENCH_DATA?: R2Bucket;
				ISR_CACHE?: KVNamespace;
				TAG_INDEX?: DurableObjectNamespace;
			};
			context?: ExecutionContext;
		}
	}
}

export {};
