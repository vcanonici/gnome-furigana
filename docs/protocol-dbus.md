# Contrato D-Bus — `io.github.vcanonici.Furigana1`

Serviço de sessão `io.github.vcanonici.Furigana`, objeto `/io/github/vcanonici/Furigana`.
Todos os métodos devolvem **uma string JSON**; os tipos estão em `service/src/protocol.rs`
(Rust) e `extension/src/core/protocol.ts` (TypeScript), que devem mudar juntos.

| Método | Entrada | Saída |
|---|---|---|
| `Analyze` | `s text` (≤ 512 pontos de código), `s context` (componente, só diagnóstico) | `Analysis` |
| `Lookup` | `s term` (forma de dicionário), `s reading` (ou vazio) | `Entry` ou `null` |
| `Health` | — | `Health` |
| `Reload` | — | `Health` (após recarregar glossários e limpar o cache) |

Erros: `org.freedesktop.DBus.Error.InvalidArgs` para texto longo demais;
`org.freedesktop.DBus.Error.Failed` para falhas internas.

## Texto já anotado

Se o texto chega com leituras inline (por exemplo, um nome vindo de um catálogo
`ja@furigana`: `設定(せってい)`), `Analyze` remove os grupos `(ひらがな)` que seguem uma
sequência de kanji e devolve:

- `plain_text`: o texto sem as leituras. **Os offsets dos tokens referem-se a ele.**
- `reading_hints_applied`: quantas leituras presentes foram reaproveitadas.

Assim nada é anotado duas vezes.

## Convenção de offsets

`start`/`end` são **pontos de código Unicode**, fim exclusivo — a mesma unidade das posições de
`Clutter.Text`. Strings JavaScript indexam UTF-16; use `core/textOffsets.ts` para converter
(𠮷 conta 1 ponto de código e 2 unidades UTF-16).

## Exemplo

```json
{"text":"電源を切る","display_inline":"電源(でんげん)を切(き)る",
 "phrase":{"term":"電源を切る","glosses":["desligar (o computador)"],"source":"glossary","language":"pt-BR"},
 "tokens":[{"surface":"電源","reading":"でんげん","start":0,"end":2,
            "segments":[{"text":"電源","reading":"でんげん"}],"lemma":"電源","part_of_speech":"substantivo",
            "meaning":{"term":"電源","glosses":["energia elétrica","alimentação"],"source":"glossary","language":"pt-BR"}}]}
```

O serviço nunca registra o texto analisado: os logs (JSON por linha no journal) contêm apenas
evento, contexto, duração e status.
