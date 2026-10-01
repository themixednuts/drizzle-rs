<script lang="ts">
	import ArchTag from './ArchTag.svelte';
	import { cn } from '#lib/utils.js';
	import type { DbVerdict } from '../../routes/home.svelte';

	/**
	 * One card per database: where drizzle-rs placed in that engine's own field, and how far it sits
	 * from the fastest raw driver on the same engine.
	 *
	 * The global table cannot answer this — drizzle-rs can be tenth overall and first on its database,
	 * and both are true — so it is answered here, once, in words, before the table.
	 */
	let {
		verdicts,
		metric,
		active,
	}: { verdicts: DbVerdict[]; metric: string; active: string | null } = $props();
</script>

<ul
	class="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-5"
	aria-label="drizzle-rs by database"
>
	{#each verdicts as verdict (verdict.db)}
		<li>
			<a
				href={verdict.href}
				aria-current={active === verdict.db ? 'true' : undefined}
				class={cn(
					'bg-card hover:bg-signal-wash flex h-full flex-col rounded-md border border-transparent px-4 pt-3.5 pb-4 transition-colors',
					active === verdict.db && 'border-signal',
				)}
			>
				<span class="flex items-baseline justify-between gap-2">
					<span class="text-lead font-semibold">{verdict.label}</span>
					<span class="text-label text-muted-foreground font-mono">{verdict.field} targets</span>
				</span>

				{#if verdict.ours}
					<span class="mt-3 flex items-baseline gap-2">
						<span class="text-figure text-signal-ink font-mono font-semibold tabular-nums">
							#{verdict.ours.position}
						</span>
						<span class="text-meta text-muted-foreground">
							of {verdict.field} · {verdict.ours.value}
						</span>
					</span>
					<span class="text-meta mt-1.5 block">
						{#if verdict.vsRaw && verdict.raw}
							<span
								class={cn(
									'font-mono font-medium tabular-nums',
									verdict.ahead && verdict.vsRaw !== '=' ? 'text-positive' : 'text-foreground',
								)}
							>
								{verdict.vsRaw === '=' ? 'level' : verdict.vsRaw}
							</span>
							<span class="text-muted-foreground">vs raw {verdict.raw.name}</span>
						{:else}
							<span class="text-muted-foreground">no raw-driver baseline on this engine</span>
						{/if}
					</span>
				{:else}
					<span class="text-meta text-muted-foreground mt-3 block">
						No drizzle-rs target here — shown so every engine is compared on the same {metric}.
					</span>
				{/if}

				<span class="mt-auto flex flex-wrap gap-x-3 gap-y-1 pt-3">
					{#each verdict.architectures as arch (arch)}
						<ArchTag {arch} quiet plain />
					{/each}
				</span>
			</a>
		</li>
	{/each}
</ul>
