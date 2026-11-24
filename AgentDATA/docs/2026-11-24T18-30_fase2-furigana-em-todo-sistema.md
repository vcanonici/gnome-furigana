# Plano — gnome-furigana fase 2: furigana em TODO o sistema

## Contexto

Com a sessão em japonês, o usuário viu dois problemas:

1. **Apps GTK sem furigana.** Configurações (設定) mostra 電源管理, 外観, 固定ワークスペース(F)… sem
   leitura. A v0.1 só cobre configurações rápidas e barra superior do Shell.
2. **O furigana some quando falta espaço.** A política atual tinha o passo "omitir". A nova regra
   do usuário: **furigana acima quando houver espaço; senão, sempre `漢字(かんじ)` na frente do
   kanji. Nunca omitir.**

Decisões do usuário: pode usar **sudo uma vez** na instalação (catálogos no sistema +
regeneração após `apt`). **Remover o atalho que força Configurações em inglês.**

Fatos verificados:
- Ubuntu 24.04: 375 catálogos `ja` em `/usr/share/locale-langpack/ja` e 24 em `/usr/share/locale/ja`.
- Só o gnome-control-center tem 1576 strings com kanji (ex.: `固定ワークスペース(_F)`, `電源管理`).
- A glibc do Ubuntu procura em `locale-langpack`.
- O `LANGUAGE=ja:en_US:en` da sessão vem de `~/.pam_environment` (escrito pelo painel Região e idioma).
- O `~/.bashrc` força o terminal em inglês. Fica intocado.

## Passo 0 — Regras e local do projeto

- Seguir `AGENTS.md` + `AgentDATA/procedures/` (já lidos: code-quality, validation,
  git-operation, legacy-and-delete, documentation, cli-tool-*, logging-and-knowledge,
  agentdata-bootstrap).
- **Mover o projeto** `~/gnome-furigana` → `~/GitRepos/gnome-furigana` com `mv`, que preserva
  `.git`, o remote e o `node_modules`. Depois:
  - conferir que nenhum caminho absoluto aponta para o local antigo;
  - rodar `npm run check`;
  - atualizar a memória `gnome-furigana-project`.
- Registrar este plano em `AgentDATA/docs/{timestamp}_fase2-furigana-em-todo-sistema.md`.
- Commits por etapa (`feat(catalog)…`, `feat(extension)…`), sempre `git add` com caminhos
  específicos e um branch `agent/<timestamp>_furigana-everywhere`.

## Passo 1 — Formato inline compartilhado (Rust, `service/src/inline_readings.rs`)

Um único módulo define o formato `漢字(かな)` nos dois sentidos:
- **render**: já existe em `alignment::render_inline` / `analysis::render_inline_text`; vai ser movido para este módulo.
- **parse**: reconhece leituras já presentes (uma sequência de kanji seguida de `(` + só hiragana + `)`)
  e devolve o texto limpo, as leituras (usadas como dica, com prioridade sobre o IPADIC) e o
  mapeamento de offsets.

Assim o catálogo e o Shell nunca anotam duas vezes. Se um texto vier `設定(せってい)` de outro
domínio, o Shell remove os parênteses e decide entre ruby e inline.

O contrato `Analyze` ganha `plain_text` e `reading_hints_applied`. O mesmo vai para
`extension/src/core/protocol.ts` e `docs/protocol-dbus.md`.

## Passo 2 — Catálogos gettext anotados (Rust, `service/src/catalog/`)

Este passo cobre Configurações, Arquivos, GTK, libadwaita… e qualquer app que use gettext.

| Módulo | Responsabilidade |
|---|---|
| `mo_file.rs` | Ler e escrever `.mo` GNU (little/big-endian; plurais separados por `\0`; `msgctxt\x04`; cabeçalho intacto; saída ordenada por msgid). |
| `message_annotation.rs` | Anotar só trechos de texto, protegendo: tags `<…>`, entidades `&…;`, placeholders printf (`%s`, `%1$s`, `%-5.2f`…), `{nome}`, `${x}`, mnemônicos `_`. Valida se o multiconjunto de placeholders e tags é idêntico antes e depois; se divergir, mantém o original. **Não anota** formatos de data/strftime (msgid com `%Y %m %d %H %M %b %a %e %p…`), porque apps fazem parse deles. |
| `catalog_build.rs` | Fontes: `locale-langpack/ja` (prioridade) + `locale/ja`. Saída: `/usr/share/locale-langpack/ja@furigana/LC_MESSAGES/<domínio>.mo`. Escrita atômica, inventário de hashes (`.furigana-inventory.json`) para só refazer o que mudou. Domínios excluídos: `gnome-shell` e `mutter`, porque o Shell é anotado dinamicamente com ruby. |

CLI: `gnome-furigana build-catalogs [--source …] [--output …] [--exclude dom,…] [--dry-run] [--json]`,
com `--help`, exit codes e registro em `AgentDATA/tools/`.

Reaproveitar: `Analyzer` (`analysis.rs`), `counters.rs`, `alignment.rs`, `Glossary`.
Os caracteres originais nunca são alterados; só se inserem `(かな)`.

## Passo 3 — Ativação na sessão e instalador (parte com sudo)

`install.sh` ganha `install_system_catalogs` (pula com `--no-system`):
- Copia o binário para `/usr/local/lib/gnome-furigana/gnome-furigana`, dono root. Root nunca
  executa o binário gravável em `~`.
- Gera os catálogos (`sudo … build-catalogs`).
- Instala o gancho `/etc/apt/apt.conf.d/99gnome-furigana`
  (`DPkg::Post-Invoke { "… build-catalogs --quiet || true"; };`).
- Registra os caminhos de sistema num manifesto separado (`system-manifest`).
  `uninstall.sh` remove esses caminhos com sudo.

Sessão: no `~/.pam_environment` (o mesmo mecanismo que o Ubuntu usa), `LANGUAGE` vira
`ja@furigana:ja:en_US:en`. Antes, faço backup do arquivo e o uninstall restaura. Se `ja@furigana`
faltar num domínio, a glibc cai para `ja`.

Configurações: remover o wrapper `env LANG=en_US…` (decisão do usuário). O backup continua guardado.

## Passo 4 — Shell: todo o Shell + nunca omitir (TypeScript)

- `core/layoutFit.ts`: remover `{form: 'none'}`. A ordem fica: ruby (escala 1 → `minScale`) →
  ruby com espaço extra (quando o componente permite) → **inline, sempre**. Ajustar `test/core.test.ts`.
- Trocar os componentes fixos por um **`ShellWideAnnotator`** sobre `Main.layoutManager.uiGroup`.
  Ele cobre barra, todos os menus, configurações rápidas, Overview (grade de apps, dash, busca,
  títulos de janelas), diálogos modais, notificações/calendário, OSD e alternador de apps.
  - Varre só subárvores mapeadas.
  - Reescaneia ao abrir menus e o Overview, e a cada 750 ms enquanto houver mudanças
    (aproveitar `findLabels`/`scan` de `labelTreeAnnotator.ts`).
  - Hover via `captured-event` no `global.stage`.
  - Política de layout por ancestral: botões de menu ganham espaço extra; barra e grade de apps
    podem crescer em largura.
- Excluir os próprios atores (overlay e tooltip marcados `furigana-*`) e `St.Entry`.
- Labels com leitura já inline vinda de outro domínio: aplicar o `plain_text` do serviço quando
  for usar ruby.
- Configurações: os interruptores por componente viram "Shell inteiro" + "apps (catálogos)" no
  schema, no painel C e no `prefs.ts`.

## Passo 5 — Teste real na VM com login gráfico

A VM é a mesma (`~/vms/gnome-furigana-test`), agora com **GDM + autologin do `tester`**,
`-vga virtio`, ainda `-display none`, e capturas por `screendump` via monitor QMP. Assim nada
aparece na tela do usuário e se testa o login real: `pam_environment`, Shell e apps GTK juntos.

Roteiro:
1. `install.sh` (com sudo) → reboot → autologin.
2. Abrir Configurações → capturar tela: 電源(でんげん)管理(かんり), 外観(がいかん),
   固定(こてい)ワークスペース(F) visíveis.
3. Abrir Arquivos → capturar tela.
4. Overview/grade de apps → ruby ou inline em todos os rótulos com kanji.
5. `run-acceptance.sh` continua passando, com um novo caso de "nunca omitir".
6. `apt-get install --reinstall gnome-control-center-data` → o gancho regera só o domínio afetado.
7. `uninstall.sh` → `.mo` originais com hash idêntico (CA-S03), `pam_environment` e atalho restaurados.

Testes unitários:
- Rust: roundtrip `.mo`, placeholders/markup/mnemônicos/plurais/strftime preservados,
  parse⇄render inline.
- TS: fit nunca omite.
- Tudo dentro de `npm run check`.

Depois de verde: push, CI, release `v0.2.0`, reinstalar no PC do usuário (avisar antes do prompt
de sudo e do logout) e atualizar `docs/compatibility-matrix.md` e os registros em `Knowledge/`.

## Riscos aceitos e limites

- Apps GTK recebem só inline (Pango não tem ruby). Hover em apps GTK fica para a fase AT-SPI.
- Texto que não vem do gettext (nomes de arquivos, conteúdo) não é anotado.
- Apps Snap/Flatpak usam catálogos próprios e ficam fora.
- Apps abertos antes do login seguinte só mudam após reiniciar.
