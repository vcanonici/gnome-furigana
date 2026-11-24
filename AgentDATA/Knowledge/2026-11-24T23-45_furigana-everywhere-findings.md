# Registro de conhecimento

## Metadata

- Timestamp: `2026-11-24T23:45:00+00:00`
- Autor: `agent`
- Assunto: fase 2 — furigana em todo o sistema (catálogos, UniDic, Shell inteiro)
- Status: `ativo`
- Retestar em: ao mudar de versão do Ubuntu/GNOME ou do Lindera

## Achado

1. **Catálogos gettext**: a glibc procura cada mensagem na ordem do `LANGUAGE` e cai para o
   próximo idioma se não achar. Por isso `ja@furigana` só precisa do cabeçalho + mensagens
   anotadas. Ubuntu procura em `/usr/share/locale-langpack`. 378 catálogos, ~93 mil mensagens, ~4 s.
2. **Sessão**: no Ubuntu 24.04 o `LANGUAGE` da sessão GNOME vem de `~/.pam_environment`
   (escrito pelo painel Região e idioma); o SSH também o lê. Mudar ali ativa `ja@furigana` no login.
3. **Dicionário**: IPADIC errava leituras comuns de UI (行 くだり, 正しく まさしく, 値 ね).
   UniDic (Lindera `embed-unidic`) acerta; binário ~195 MB, RSS ~17 MB. Colunas usadas: 6
   (leitura do lexema), 9 (pronúncia, com ー), 10 (forma base na grafia original). Rendaku
   (土曜日) depende da palavra inteira: resolvido pelo glossário curado.
4. **`disabled-extensions` vence `enabled-extensions`** no GNOME 46: o instalador remove da primeira.
5. **Shell**: com um menu aberto (grab), o stage não recebe eventos; ponteiro via
   `Meta.CursorTracker.get_for_display(global.display)` sinal `position-invalidated`.
6. **`GLib.PRIORITY_LOW`** pode nunca rodar enquanto o Shell redesenha: usar `PRIORITY_DEFAULT`.
7. Rótulos do Shell somem o tempo todo (notificações expiram): nunca resetar o hover global
   por destruição de outro rótulo.
8. **`gdbus call` trata argumentos como texto GVariant** e consome barras invertidas: código
   passado ao `Eval` não pode depender de `\(`, `\p{…}`; usar intervalos literais.
9. GDM na VM: `systemctl enable gdm` falha; usar o link `display-manager.service` → `gdm3.service`.
   Autologin acontece uma vez por boot.

## Evidencia

Capturas na VM (`~/vms/gnome-furigana-test/shots/`): Configurações e Arquivos com leituras
inline; relógio `11月(じゅういちがつ)24日(にじゅうよっか)`; notificação com ruby; suíte de
aceitação 2/2 verde.

## Consequencia operacional

Novos domínios/vocabulário: atualizar `data/ui-glossary-pt_BR.tsv` (leituras revisadas têm
prioridade). Testes visuais sempre com login real na VM.
