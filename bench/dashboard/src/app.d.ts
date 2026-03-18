/// <reference path="./cloudflare.d.ts" />

import type { ISRRequestScope } from 'cloudflare-isr';

declare global {
	namespace App {
		interface Locals {
			isr: ISRRequestScope;
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
