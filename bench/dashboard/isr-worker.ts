export { ISRTagIndexDO } from "cloudflare-isr";

export default {
	async fetch() {
		return new Response("drizzle-bench-isr DO host", { status: 200 });
	},
};
