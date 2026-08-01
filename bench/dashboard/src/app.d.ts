import './cloudflare.d.ts';

declare global {
	namespace App {
		interface Platform {
			env?: {
				BENCH_DATA?: R2Bucket;
			};
			/** Execution context, used to finish cache writes after the response is sent. */
			ctx?: ExecutionContext;
			/** The Workers Cache API. Absent outside the Workers runtime. */
			caches?: CacheStorage & { default: Cache };
		}
	}
}

export {};
