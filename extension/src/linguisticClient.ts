/**
 * Asynchronous client of the linguistic service. Never blocks the Shell: calls have a
 * short timeout, and any failure resolves to `null` so callers keep the original text
 * (spec CA-S01).
 */

import Gio from 'gi://Gio';
import GLib from 'gi://GLib';

import {type Analysis, isAnalysis} from './core/protocol.js';
import {codePointLength} from './core/textOffsets.js';

const BUS_NAME = 'io.github.vcanonici.Furigana';
const OBJECT_PATH = '/io/github/vcanonici/Furigana';
const INTERFACE = 'io.github.vcanonici.Furigana1';
const CALL_TIMEOUT_MS = 500;
const MAX_CACHED = 512;
const MAX_TEXT_CHARS = 512;

/** Caches analyses of UI strings in memory for the Shell session. */
export class LinguisticClient {
    private readonly cache = new Map<string, Analysis>();
    private readonly cancellable = new Gio.Cancellable();
    private failures = 0;

    /** Analyses `text`; resolves to null when the service is unavailable or the reply is invalid. */
    async analyze(text: string, context: string): Promise<Analysis | null> {
        const cached = this.cache.get(text);
        if (cached)
            return cached;
        if (codePointLength(text) > MAX_TEXT_CHARS)
            return null;
        const analysis = await this.call(text, context);
        if (analysis)
            this.remember(text, analysis);
        return analysis;
    }

    /** Number of failed calls since enable, for diagnostics. */
    get failureCount(): number {
        return this.failures;
    }

    /** Cancels in-flight calls and drops the cache. */
    destroy(): void {
        this.cancellable.cancel();
        this.cache.clear();
    }

    private remember(text: string, analysis: Analysis): void {
        if (this.cache.size >= MAX_CACHED) {
            const oldest = this.cache.keys().next().value;
            if (oldest !== undefined)
                this.cache.delete(oldest);
        }
        this.cache.set(text, analysis);
    }

    private call(text: string, context: string): Promise<Analysis | null> {
        return new Promise(resolve => {
            Gio.DBus.session.call(BUS_NAME, OBJECT_PATH, INTERFACE, 'Analyze',
                new GLib.Variant('(ss)', [text, context]), new GLib.VariantType('(s)'),
                Gio.DBusCallFlags.NONE, CALL_TIMEOUT_MS, this.cancellable,
                (connection, result) => { resolve(this.decode(connection, result)); });
        });
    }

    private decode(connection: Gio.DBusConnection | null, result: Gio.AsyncResult): Analysis | null {
        try {
            const reply: unknown = connection?.call_finish(result).deepUnpack();
            if (!Array.isArray(reply) || typeof reply[0] !== 'string')
                throw new Error('Analyze reply is not (s)');
            const value: unknown = JSON.parse(reply[0]);
            if (isAnalysis(value))
                return value;
            throw new Error('unexpected Analyze reply shape');
        } catch (error) {
            this.failures++;
            if (!this.cancellable.is_cancelled())
                console.debug(`[gnome-furigana] analyze failed: ${error instanceof Error ? error.message : String(error)}`);
            return null;
        }
    }
}
