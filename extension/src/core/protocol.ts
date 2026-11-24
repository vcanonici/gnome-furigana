/**
 * JSON contract of the `io.github.vcanonici.Furigana1` D-Bus interface.
 * Mirrors `service/src/protocol.rs`; offsets are Unicode code points, end-exclusive,
 * the same unit as `Clutter.Text` cursor positions.
 */

/** Provenance of a meaning. */
export type MeaningSource = 'override' | 'glossary' | 'jmdict';

/** A meaning; `language` is the language of `glosses` (`pt-BR` or `en`). */
export interface Meaning {
    term: string;
    reading: string | null;
    glosses: string[];
    part_of_speech: string | null;
    source: MeaningSource;
    language: 'pt-BR' | 'en';
}

/** A piece of a word; `reading` is null for kana that needs no furigana. */
export interface Segment {
    text: string;
    reading: string | null;
}

/** One analysed word. */
export interface Token {
    surface: string;
    reading: string | null;
    start: number;
    end: number;
    segments: Segment[];
    lemma: string;
    part_of_speech: string;
    meaning: Meaning | null;
}

/** Result of `Analyze`. Token offsets refer to `plain_text`. */
export interface Analysis {
    text: string;
    /** `text` without readings it already carried (`設定(せってい)` → `設定`). */
    plain_text: string;
    reading_hints_applied: number;
    tokens: Token[];
    phrase: Meaning | null;
    display_inline: string;
}

/** Narrow runtime check of a decoded `Analyze` reply; rejects anything unexpected. */
export function isAnalysis(value: unknown): value is Analysis {
    if (typeof value !== 'object' || value === null)
        return false;
    const candidate = value as Partial<Analysis>;
    return typeof candidate.text === 'string' &&
        typeof candidate.plain_text === 'string' &&
        typeof candidate.display_inline === 'string' &&
        Array.isArray(candidate.tokens) &&
        candidate.tokens.every(token => typeof token.start === 'number' && typeof token.end === 'number' &&
            Array.isArray(token.segments));
}
