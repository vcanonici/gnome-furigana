/**
 * Fit policy: furigana above the kanji when there is room (shrinking down to
 * `minScale`, then asking the component for a little extra room), otherwise the
 * reading goes inline after the kanji — `漢字(かんじ)`. Never omitted: a label that
 * grows or ellipsizes is better than kanji without a reading.
 */

/** Geometry measured for one label, in logical pixels. */
export interface FitInput {
    /** Free vertical space between the label's top and the nearest obstacle above it. */
    spaceAbove: number;
    /** Extra space the component can add above the label (padding), 0 if none. */
    extraRoom: number;
    /** Height of the ruby line at scale 1. */
    rubyHeight: number;
    /** Smallest ruby scale allowed (default 0.8). */
    minScale: number;
    /** User preference. */
    preferredForm: 'ruby' | 'inline';
}

/** What to render. */
export type FitDecision =
    | {form: 'ruby'; scale: number; needsRoom: boolean}
    | {form: 'inline'};

const SCALE_STEP = 0.05;
// Ruby glyphs have internal leading; allow them to overlap the obstacle slightly.
const OVERLAP_TOLERANCE = 2;

function largestRubyScale(input: FitInput, space: number): number | null {
    for (let scale = 1; scale >= input.minScale - 1e-9; scale -= SCALE_STEP) {
        if (input.rubyHeight * scale <= space + OVERLAP_TOLERANCE)
            return Math.round(scale * 100) / 100;
    }
    return null;
}

/** Chooses the rendering for one label. Pure; unit-tested. */
export function decideFit(input: FitInput): FitDecision {
    if (input.preferredForm === 'inline')
        return {form: 'inline'};
    const scale = largestRubyScale(input, input.spaceAbove);
    if (scale !== null)
        return {form: 'ruby', scale, needsRoom: false};
    const roomyScale = input.extraRoom > 0 ? largestRubyScale(input, input.spaceAbove + input.extraRoom) : null;
    if (roomyScale !== null)
        return {form: 'ruby', scale: roomyScale, needsRoom: true};
    return {form: 'inline'};
}
