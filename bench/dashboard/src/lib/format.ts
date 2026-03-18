export function fmtRps(n: number): string {
	if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + 'M';
	if (n >= 1_000) return (n / 1_000).toFixed(1) + 'k';
	return n.toFixed(0);
}

export function fmtLatency(ms: number): string {
	if (ms >= 1_000) return (ms / 1_000).toFixed(2) + 's';
	if (ms >= 1) return ms.toFixed(1) + 'ms';
	return (ms * 1_000).toFixed(0) + 'us';
}

export function fmtPct(n: number): string {
	return (n * 100).toFixed(2) + '%';
}

export function fmtDelta(pct: number): string {
	const sign = pct >= 0 ? '+' : '';
	return sign + (pct * 100).toFixed(1) + '%';
}

export function fmtCpu(n: number): string {
	return n.toFixed(1) + '%';
}

export function fmtDate(iso: string): string {
	const d = new Date(iso);
	return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric' }) +
		' ' +
		d.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit', hour12: false });
}

export function fmtDuration(start: string, end: string): string {
	const ms = new Date(end).getTime() - new Date(start).getTime();
	const s = Math.floor(ms / 1000);
	if (s < 60) return s + 's';
	const m = Math.floor(s / 60);
	const rem = s % 60;
	if (m < 60) return m + 'm ' + rem + 's';
	const h = Math.floor(m / 60);
	return h + 'h ' + (m % 60) + 'm';
}

export function shortHash(git: string): string {
	return git.slice(0, 7);
}
