/**
 * Code point ↔ UTF-16 conversions. The service and `Clutter.Text` count code points;
 * JavaScript strings index UTF-16 units, which differ for characters outside the BMP (𠮷).
 */

import type {Segment, Token} from './protocol.js';

/** A code-point range `[start, end)` of one annotated (kanji) segment. */
export interface RubySpan {
    start: number;
    end: number;
    reading: string;
}

/** Number of code points in `text`. */
export function codePointLength(text: string): number {
    let length = 0;
    for (let index = 0; index < text.length; index++, length++) {
        const unit = text.charCodeAt(index);
        // A high surrogate followed by a low surrogate is one code point.
        if (unit >= 0xD800 && unit <= 0xDBFF && index + 1 < text.length)
            index++;
    }
    return length;
}

/** Returns the token covering code point `position`, if any. */
export function tokenAt(tokens: readonly Token[], position: number): Token | null {
    return tokens.find(token => position >= token.start && position < token.end) ?? null;
}

/** Flattens tokens into code-point spans of segments that carry a reading. */
export function rubySpans(tokens: readonly Token[]): RubySpan[] {
    const spans: RubySpan[] = [];
    for (const token of tokens) {
        let start = token.start;
        for (const segment of token.segments) {
            const end = start + codePointLength(segment.text);
            if (segment.reading !== null)
                spans.push({start, end, reading: segment.reading});
            start = end;
        }
    }
    return spans;
}

/** Inline rendering `漢字(かんじ)` of segments, identical to the service's `display_inline`. */
export function renderInline(segments: readonly Segment[]): string {
    return segments.map(segment => segment.reading ? `${segment.text}(${segment.reading})` : segment.text).join('');
}

function pushRange(positions: number[], start: number, end: number): void {
    for (let position = start; position < end; position++)
        positions.push(position);
}

/**
 * Maps each code point of the inline rendering back to the original text, so a hit
 * inside `電源(でんげん)` still resolves to 電源. Characters of an added reading map to
 * the last original code point of their segment; text between words (spaces the
 * analyzer skipped) maps to itself, as the service copies it unchanged.
 */
export function inlineToOriginalPositions(text: string, tokens: readonly Token[]): number[] {
    const positions: number[] = [];
    let cursor = 0;
    for (const token of tokens) {
        pushRange(positions, cursor, token.start);
        cursor = token.end;
        let original = token.start;
        for (const segment of token.segments) {
            const length = codePointLength(segment.text);
            for (let offset = 0; offset < length; offset++)
                positions.push(original + offset);
            if (segment.reading !== null)
                positions.push(...Array<number>(codePointLength(segment.reading) + 2).fill(original + length - 1));
            original += length;
        }
    }
    pushRange(positions, cursor, codePointLength(text));
    return positions;
}
