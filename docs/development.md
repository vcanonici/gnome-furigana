# Desenvolvimento

```text
extension/        TypeScript → build/extension/<uuid> (tsc); core/ é puro e testado com node:test
service/          Rust: morphology (Lindera), alignment, glossary, jmdict_index, analysis, dbus_interface
settings-panel/   painel C/GTK4 do gnome-control-center e build-fork.sh
data/             glossário pt-BR curado (embutido no binário do serviço)
packaging/        unidades systemd/D-Bus e stage-release.sh
tests/shell/      aceitação num GNOME Shell 46 headless (apenas VM)
```

`npm run check` executa os targets Nx `lint`, `test` e `build` (o mesmo que a CI).

## Testar no Shell sem sair da sessão

Não teste num desktop real: use uma VM descartável com Ubuntu 24.04 e
`tests/shell/run-acceptance.sh`, que inicia `gnome-shell --headless` com locale japonês num D-Bus
privado, abre as configurações rápidas, move um ponteiro virtual e verifica os critérios de aceite.
O harness instala uma extensão **só de teste** (`unsafe-mode@furigana-tests`) para liberar
`org.gnome.Shell.Eval`; nunca a instale numa sessão real.

Armadilhas encontradas (ver `AgentDATA/Knowledge/`):

- Um Shell morto durante o arranque deixa `$XDG_RUNTIME_DIR/gnome-shell-disable-extensions`; o
  próximo Shell inicia com todas as extensões desativadas.
- `PrivateNetwork=` não funciona em unidades `systemd --user` do Ubuntu 24.04; usamos
  `RestrictAddressFamilies=AF_UNIX` (seccomp).
- `@girs/gnome-shell@46` tem dependências com faixas que puxam gerações incompatíveis; o
  `package.json` fixa todas as `@girs/*` na geração `4.0.0-beta.12` via `overrides`.

## Release

Tag `vX.Y.Z` → `.github/workflows/release.yml` compila o fork das Configurações num contêiner
`ubuntu:24.04`, roda `npm run check`, monta o tarball com `packaging/stage-release.sh` e publica o
release com checksum SHA-256 (verificado pelo `install.sh`).
