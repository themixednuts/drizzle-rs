import { fmtCpu, fmtGb } from './format';
import type { Manifest } from './types';

/**
 * The run's configuration, in the words a reader uses.
 *
 * The manifest stores this as enums and spec hashes — `ramping-vus`, `target_process_tree`,
 * `sha256:56ac58…` — which are the runner's vocabulary, not anyone's. Printing them verbatim in a
 * table made a reader translate on every row, and the workload hash in particular is 71 characters
 * of provenance that answers no question a person reading this page is asking.
 *
 * Each fact is one row: a label and a value, never two label/value pairs zig-zagged across four
 * columns.
 */
export interface RunFact {
	label: string;
	value: string;
	/** Longer explanation for a tooltip, when the plain wording still hides a decision. */
	hint?: string;
}

function executorPhrase(executor: string, stages: number): string {
	if (executor === 'ramping-vus') {
		return stages > 1
			? `rises in ${stages} steps, then holds`
			: 'rises to a target level, then holds';
	}
	if (executor === 'constant-vus') return 'held at a constant level throughout';
	return executor;
}

function cpuScopePhrase(scope: string | undefined): string {
	if (scope === 'host') return 'the whole machine';
	if (scope === 'target_process_tree') return "the server's process tree";
	if (scope === 'cgroup') return "the server's cgroup";
	return scope ?? 'not declared';
}

function memoryScopePhrase(scope: string | undefined): string {
	if (scope === 'target_process_tree') return "the server's process tree";
	if (scope === 'host') return 'the whole machine';
	if (scope === 'cgroup') return "the server's cgroup";
	return scope ?? 'not declared';
}

function networkScopePhrase(scope: string | undefined): string {
	if (!scope || scope === 'unmeasured') return 'not measured';
	if (scope === 'host') return 'the whole machine';
	return scope;
}

/**
 * The facts that belong in the "About this run" card.
 *
 * Deliberately excludes anything already on the meta strip under the title — commit, start,
 * duration, trials, hardware — because repeating them is what turned this section into a wall.
 */
export function runFacts(manifest: Manifest): RunFact[] {
	const load = manifest.load;
	const dataset = manifest.dataset;
	const runner = manifest.runner;
	const metrics = runner.metrics;
	const facts: RunFact[] = [];

	facts.push({
		label: 'Load',
		value: `${executorPhrase(load.executor, load.stages)} — ${load.duration_s}s at up to ${load.max_vus.toLocaleString()} concurrent clients`,
	});

	facts.push({
		label: 'Requests',
		value: `${load.requests.toLocaleString()} generated from seed ${manifest.seed}`,
		hint: 'The request list is generated once from this seed, so every target answers exactly the same requests in the same order.',
	});

	if (load.pacing) {
		facts.push({
			label: 'Pacing',
			value: load.pacing,
			hint: 'Named pacing profile: the request rate is shaped rather than sent as fast as possible, so a fast target is not rewarded for burning the load generator’s own CPU.',
		});
	}

	facts.push({
		label: 'Dataset',
		value: `${dataset.orders.toLocaleString()} orders, ${dataset.customers.toLocaleString()} customers, ${dataset.products.toLocaleString()} products, ${dataset.suppliers.toLocaleString()} suppliers, ${dataset.details_per_order} line items per order`,
	});

	facts.push({
		label: 'Machine',
		value: `${runner.cpu}, ${runner.cores} cores, ${fmtGb(runner.mem_gb)} memory`,
	});

	facts.push({
		label: 'CPU measured across',
		value: cpuScopePhrase(metrics?.cpu_scope),
		hint: 'The load generator, the server and its database share this machine, so CPU is a whole-machine figure and is only comparable between targets in this same run.',
	});

	facts.push({
		label: 'Memory measured on',
		value: memoryScopePhrase(metrics?.memory_scope),
		hint: 'A database running as a separate service is only included when it is a child of the server process.',
	});

	facts.push({ label: 'Network measured', value: networkScopePhrase(metrics?.network_scope) });

	const peak = runner.headroom;
	const parts = [`${fmtCpu(peak.cpu_peak)} on the busiest core`];
	if (peak.cpu_mean_peak != null) parts.push(`${fmtCpu(peak.cpu_mean_peak)} averaged across cores`);
	facts.push({
		label: 'Peak CPU',
		value: parts.join(', '),
		hint: 'How loaded the machine got at its busiest moment. This is load, not spare capacity — a run at 100% was saturated.',
	});

	return facts;
}
