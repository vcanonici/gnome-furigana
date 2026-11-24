/**
 * Fit policy (spec §8.3): prefer ruby above the label, shrink the ruby down to
 * `minScale`, then fall back to inline text, and finally omit the annotation
 * rather than truncating the original command.
 */

/** Geometry measured for one label, in logical pixels. */
export interface FitInput {
    /** Free vertical space between the label's top and the nearest obstacle above it. */
    spaceAbove: number;
    /** Extra space the component can add above the label (padding), 0 if none. */
    extraRoom: number;
    /** Height of the ruby line at scale 1. */
    rubyHeight: number;
    /** Width available to the label. */
    availableWidth: number;
    /** Natural width of the label when rendered inline (`漢字(かんじ)`). */
    inlineWidth: number;
    /** Smallest ruby scale allowed (default 0.8). */
    minScale: number;
    /** User preference. */
    preferredForm: 'ruby' | 'inline';
}

/** What to render. */
export type FitDecision =
    | {form: 'ruby'; scale: number; needsRoom: boolean}
    | {form: 'inline'}
    | {form: 'none'};

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
    const inlineFits = input.inlineWidth <= input.availableWidth;
    if (input.preferredForm === 'inline')
        return inlineFits ? {form: 'inline'} : {form: 'none'};
    const scale = largestRubyScale(input, input.spaceAbove);
    if (scale !== null)
        return {form: 'ruby', scale, needsRoom: false};
    // Growing the component a little keeps furigana readable at full size.
    const roomyScale = input.extraRoom > 0 ? largestRubyScale(input, input.spaceAbove + input.extraRoom) : null;
    if (roomyScale !== null)
        return {form: 'ruby', scale: roomyScale, needsRoom: true};
    return inlineFits ? {form: 'inline'} : {form: 'none'};
}
