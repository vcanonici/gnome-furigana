import {test} from 'node:test';
import assert from 'node:assert/strict';

import {HoverStateMachine, type Scheduler} from '../src/core/hoverStateMachine.js';
import {decideFit, type FitInput} from '../src/core/layoutFit.js';
import {codePointLength, inlineToOriginalPositions, renderInline, rubySpans, tokenAt} from '../src/core/textOffsets.js';
import {UNAVAILABLE, buildTooltipContent} from '../src/core/tooltipContent.js';
import {isAnalysis, type Token} from '../src/core/protocol.js';

/** Deterministic clock: timers fire only when `advance` passes their deadline. */
class FakeClock implements Scheduler {
    now = 0;
    private nextHandle = 1;
    private timers = new Map<number, {at: number; callback: () => void}>();

    schedule(delayMs: number, callback: () => void): number {
        const handle = this.nextHandle++;
        this.timers.set(handle, {at: this.now + delayMs, callback});
        return handle;
    }

    cancel(handle: number): void {
        this.timers.delete(handle);
    }

    advance(ms: number): void {
        this.now += ms;
        for (const [handle, timer] of [...this.timers]) {
            if (timer.at <= this.now) {
                this.timers.delete(handle);
                timer.callback();
            }
        }
    }
}

function hoverHarness() {
    const clock = new FakeClock();
    const events: string[] = [];
    const machine = new HoverStateMachine<string>(clock, {
        show: target => events.push(`show:${target}`),
        hide: () => events.push('hide'),
    }, 2000);
    return {clock, events, machine};
}

const settingsToken: Token = {
    surface: '設定', reading: 'せってい', start: 0, end: 2, lemma: '設定', part_of_speech: 'substantivo',
    segments: [{text: '設定', reading: 'せってい'}],
    meaning: {term: '設定', reading: 'せってい', glosses: ['configurações', 'definir'], part_of_speech: 'substantivo',
        source: 'glossary', language: 'pt-BR'},
};

test('CA-H01/H02: tooltip appears only after 2000 ms on the same word, despite small moves', () => {
    const {clock, events, machine} = hoverHarness();
    machine.enter('label:0', '設定');
    clock.advance(1900);
    machine.enter('label:0', '設定');
    assert.deepEqual(events, []);
    clock.advance(100);
    assert.deepEqual(events, ['show:設定']);
});

test('CA-H03: changing word restarts the 2000 ms count', () => {
    const {clock, events, machine} = hoverHarness();
    machine.enter('label:0', '電源');
    clock.advance(1500);
    machine.enter('label:3', '切る');
    clock.advance(1500);
    assert.deepEqual(events, []);
    clock.advance(500);
    assert.deepEqual(events, ['show:切る']);
});

test('CA-H04: leaving cancels the timer and hides a visible tooltip', () => {
    const {clock, events, machine} = hoverHarness();
    machine.enter('a', 'A');
    machine.reset();
    clock.advance(5000);
    assert.deepEqual(events, []);
    machine.enter('b', 'B');
    clock.advance(2000);
    machine.reset();
    assert.deepEqual(events, ['show:B', 'hide']);
    assert.equal(machine.isShowing, false);
});

test('offsets are code points: 𠮷 counts as one position', () => {
    assert.equal(codePointLength('𠮷野家'), 3);
    const token: Token = {...settingsToken, surface: '行く', start: 4, end: 6,
        segments: [{text: '行', reading: 'い'}, {text: 'く', reading: null}]};
    assert.equal(tokenAt([token], 5), token);
    assert.equal(tokenAt([token], 6), null);
    assert.deepEqual(rubySpans([token]), [{start: 4, end: 5, reading: 'い'}]);
    assert.equal(renderInline(token.segments), '行(い)く');
});

const fit: FitInput = {spaceAbove: 12, extraRoom: 0, rubyHeight: 12, minScale: 0.8, preferredForm: 'ruby'};

test('fit policy: ruby, shrunk ruby, ruby with room, otherwise always inline (never omitted)', () => {
    assert.deepEqual(decideFit(fit), {form: 'ruby', scale: 1, needsRoom: false});
    assert.deepEqual(decideFit({...fit, spaceAbove: 9}), {form: 'ruby', scale: 0.9, needsRoom: false});
    assert.deepEqual(decideFit({...fit, spaceAbove: 4, extraRoom: 8}), {form: 'ruby', scale: 1, needsRoom: true});
    assert.deepEqual(decideFit({...fit, spaceAbove: 4}), {form: 'inline'});
    // No room at all, not even for inline width: the reading is still shown inline.
    assert.deepEqual(decideFit({...fit, spaceAbove: 0}), {form: 'inline'});
    assert.deepEqual(decideFit({...fit, preferredForm: 'inline'}), {form: 'inline'});
});

test('CA-H05: tooltip for 設定 shows reading and Configurações; unknown words say so', () => {
    const content = buildTooltipContent(settingsToken, null);
    assert.equal(content.heading, '設定  (せってい)');
    assert.equal(content.senses, 'configurações; definir');
    assert.equal(buildTooltipContent({...settingsToken, meaning: null}, null).senses, UNAVAILABLE);
});

test('protocol guard rejects malformed replies', () => {
    assert.equal(isAnalysis({text: 'x', plain_text: 'x', reading_hints_applied: 0, display_inline: 'x', tokens: []}), true);
    assert.equal(isAnalysis({text: 'x', display_inline: 'x', tokens: []}), false);
    assert.equal(isAnalysis({text: 'x', tokens: 'nope'}), false);
    assert.equal(isAnalysis(null), false);
});

test('inline positions map back to the original word', () => {
    const tokens: Token[] = [
        {...settingsToken, surface: '電源', start: 0, end: 2, segments: [{text: '電源', reading: 'でんげん'}]},
        {...settingsToken, surface: 'を', start: 2, end: 3, segments: [{text: 'を', reading: null}]},
    ];
    // 電源(でんげん)を → 電 源 ( で ん げ ん ) を
    assert.deepEqual(inlineToOriginalPositions('電源を', tokens), [0, 1, 1, 1, 1, 1, 1, 1, 2]);
    // Text the analyzer skipped (a space) and trailing text map to themselves.
    const spaced = [{...tokens[0], start: 1, end: 3} as Token];
    assert.deepEqual(inlineToOriginalPositions(' 電源 x', spaced), [0, 1, 2, 2, 2, 2, 2, 2, 2, 3, 4]);
});
