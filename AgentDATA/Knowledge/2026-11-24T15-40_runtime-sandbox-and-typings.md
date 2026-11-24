# Registro de conhecimento

## Metadata

- Timestamp: `2026-11-24T15:40:00+00:00`
- Autor: `agent`
- Assunto: sandbox do serviço systemd --user, tipagens GJS e build do g-c-c
- Status: `ativo`
- Retestar em: ao atualizar Ubuntu, `@girs/*` ou gnome-control-center

## Achado

1. Em unidades `systemd --user` do Ubuntu 24.04, `PrivateNetwork=yes` é ignorado ("proceeding
   without"); `RestrictAddressFamilies=AF_UNIX` + `NoNewPrivileges=yes` funciona (AF_INET → errno 97).
2. `@girs/gnome-shell@46.0.2` declara dependências `^…-4.0.0-beta.12` que o npm resolve para
   gerações diferentes (até rc.17), duplicando `GLib`/`Pango` com tipos incompatíveis. Fixar todas as
   `@girs/*` em `4.0.0-beta.12` com `overrides` no `package.json`.
3. Assinaturas de sinais do Shell tipadas como `boolean | undefined`: handlers precisam retornar
   `undefined` explicitamente sob `typescript-eslint` estrito.
4. O loader de painéis do g-c-c 46 procura `gnome-<id>-panel.desktop` via `GDesktopAppInfo`, que
   inclui `~/.local/share/applications`: o fork funciona instalado só por usuário.
5. Passar `-DSSHD_SERVICE="ssh.service"` em `-Dc_args` perde as aspas; usar `CFLAGS` (meson faz
   shlex) com `\"`.
6. Lindera 7 com `embed-ipadic` dá leituras corretas para os casos da RFC (binário ~50 MB, RSS ~7 MB);
   IPADIC separa compostos (拡張|機能), resolvido juntando palavras com kanji conhecidas no glossário.

## Consequencia operacional

Manter os `overrides` de `@girs`, a unidade com seccomp e o `build-fork.sh` com `CFLAGS`.
