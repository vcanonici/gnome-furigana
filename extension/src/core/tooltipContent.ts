/** Builds the text shown in the translation card (spec §7.2), in pt-BR. */

import type {Meaning, Token} from './protocol.js';

/** Shown when no source knows the word; never replaced by a guess. */
export const UNAVAILABLE = 'Tradução não disponível';
const MAX_SENSES = 3;

const SOURCE_LABELS: Record<Meaning['source'], string> = {
    override: 'seu vocabulário',
    glossary: 'glossário',
    jmdict: 'dicionário JMdict (inglês)',
};

/** Lines of the card, top to bottom. */
export interface TooltipContent {
    heading: string;
    senses: string;
    details: string;
    phrase: string | null;
}

/** Formats a word and, when known, the meaning of the whole UI expression. */
export function buildTooltipContent(token: Token, phrase: Meaning | null): TooltipContent {
    const reading = token.reading ? `  (${token.reading})` : '';
    const meaning = token.meaning;
    const lemmaNote = token.lemma !== token.surface ? ` · forma de dicionário ${token.lemma}` : '';
    return {
        heading: `${token.surface}${reading}`,
        senses: meaning ? meaning.glosses.slice(0, MAX_SENSES).join('; ') : UNAVAILABLE,
        details: meaning
            ? `${meaning.part_of_speech ?? token.part_of_speech} · ${SOURCE_LABELS[meaning.source]}${lemmaNote}`
            : token.part_of_speech,
        phrase: phrase && phrase.term !== token.surface ? `${phrase.term}: ${phrase.glosses[0] ?? ''}` : null,
    };
}
