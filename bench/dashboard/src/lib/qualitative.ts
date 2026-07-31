/**
 * Prose-shaped target attributes, reduced to something a layout can hold.
 *
 * `sql_variant` is the motivating case: the runner writes a full explanatory sentence — the longest
 * in the current data is 348 characters — and a metric grid rendered it as if it were a number,
 * so a five-line paragraph sat in a column next to MEMORY and ERRORS. The rule this module exists
 * to support is that a grid cell holds a value and never a sentence: qualitative attributes get a
 * short leading clause on one quiet footnote line, and the full text stays one hover away.
 */

/**
 * Where a leading clause ends.
 *
 * Splitting on `;` alone is not enough for the real strings: two of the seven in the current data
 * have no semicolon at all, and two more put theirs 120+ characters in. Each of these punctuation
 * marks ends a clause, so the first one that appears is the end of the opening statement.
 */
const BOUNDARY = /[;:—]|,\s/g;

/** A one-line footnote is about this many characters at the note-line size. */
const ONE_LINE = 80;

export interface QualitativeNote {
	/** The short form to display. */
	short: string;
	/** The complete original text, for the tooltip. */
	full: string;
	/** True when `short` is genuinely shorter, i.e. the tooltip carries something new. */
	elided: boolean;
}

/** Every prefix of `text` that ends at a clause boundary, shortest first, then the whole string. */
function clauses(text: string): string[] {
	const out: string[] = [];
	BOUNDARY.lastIndex = 0;
	let match: RegExpExecArray | null;
	while ((match = BOUNDARY.exec(text)) !== null) {
		const head = text.slice(0, match.index).trim();
		if (head && head !== out.at(-1)) out.push(head);
	}
	const whole = text.trim();
	if (whole && whole !== out.at(-1)) out.push(whole);
	return out;
}

function note(full: string, short: string): QualitativeNote {
	const trimmed = full.trim();
	return { short, full: trimmed, elided: short !== trimmed };
}

/** The short form of a single attribute, with no sibling context to disambiguate against. */
export function summarize(text: string): QualitativeNote {
	const options = clauses(text);
	const first = options[0] ?? text.trim();
	return note(text, first.length <= ONE_LINE ? first : truncate(first));
}

function truncate(text: string): string {
	if (text.length <= ONE_LINE) return text;
	const cut = text.lastIndexOf(' ', ONE_LINE);
	return `${text.slice(0, cut > 40 ? cut : ONE_LINE).trimEnd()}…`;
}

/**
 * Short forms for a set of attributes shown together, keyed by the full text.
 *
 * Two different explanations can open with the same clause — the SQLite and Turso drizzle builders
 * both begin "drizzle-rs select builder on every route" — and collapsing both to that would claim
 * they are the same thing. So when a clause is ambiguous within the set, this walks out to the next
 * boundary for exactly the strings that collide, and only while the result still fits on one line.
 * Identical inputs are not ambiguous: they collapse to the same short form because they *are* the
 * same, which is correct.
 */
export function summarizeAll(texts: readonly string[]): Map<string, QualitativeNote> {
	const distinct = [...new Set(texts.map((text) => text.trim()).filter(Boolean))];
	const levels = new Map<string, number>(distinct.map((text) => [text, 0]));
	const options = new Map<string, string[]>(distinct.map((text) => [text, clauses(text)]));

	const shortOf = (text: string): string => {
		const list = options.get(text) ?? [text];
		return list[Math.min(levels.get(text) ?? 0, list.length - 1)] ?? text;
	};

	// Walk colliding entries out one boundary at a time. Bounded by the longest clause list, so it
	// terminates even if two strings stay ambiguous to the end.
	const maxDepth = Math.max(1, ...[...options.values()].map((list) => list.length));
	for (let pass = 0; pass < maxDepth; pass += 1) {
		const byShort = new Map<string, string[]>();
		for (const text of distinct) {
			const key = shortOf(text);
			byShort.set(key, [...(byShort.get(key) ?? []), text]);
		}

		let moved = false;
		for (const group of byShort.values()) {
			if (group.length < 2) continue;
			for (const text of group) {
				const list = options.get(text) ?? [];
				const next = (levels.get(text) ?? 0) + 1;
				// Only extend while the longer form still reads as one line; past that an ambiguous
				// short label plus a tooltip beats a wrapped paragraph.
				if (next < list.length && list[next].length <= ONE_LINE) {
					levels.set(text, next);
					moved = true;
				}
			}
		}
		if (!moved) break;
	}

	return new Map(
		distinct.map((text) => {
			const short = shortOf(text);
			return [text, note(text, short.length <= ONE_LINE ? short : truncate(short))] as const;
		}),
	);
}
