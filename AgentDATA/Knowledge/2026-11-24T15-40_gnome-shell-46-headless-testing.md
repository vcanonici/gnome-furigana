# Registro de conhecimento

## Metadata

- Timestamp: `2026-11-24T15:40:00+00:00`
- Autor: `agent`
- Assunto: testar extensões do GNOME Shell 46 em VM headless
- Status: `ativo`
- Retestar em: ao mudar de versão do GNOME Shell

## Achado

1. `gnome-shell --headless --wayland --virtual-monitor 1280x800` dentro de `dbus-run-session` sobe o
   Shell 46 sem GPU nem GDM, com locale japonês via `LANG=ja_JP.UTF-8`.
2. O Shell 46 não tem `--unsafe-mode`; `org.gnome.Shell.Eval` só responde com
   `global.context.unsafe_mode = true`, que uma extensão de teste pode ligar no `enable()`.
3. `Eval` roda `JSON.stringify(await eval(code))` no escopo de `shellDBus.js`: `Main` está em escopo.
4. Um Shell morto durante o arranque deixa `$XDG_RUNTIME_DIR/gnome-shell-disable-extensions`; o
   próximo Shell desativa **todas** as extensões. Apagar o arquivo antes de cada execução de teste.
5. Serviços ativados pelo D-Bus privado herdam o stdio de quem lançou `dbus-run-session`: via SSH,
   redirecionar tudo para arquivo senão a sessão SSH nunca fecha.
6. `pkill -f <padrão>` via SSH mata o próprio comando SSH quando o padrão aparece na linha de comando.
7. Instalar `gnome-shell` num cloud image puxa o GDM; desativar `gdm3` na VM de teste.
8. Chamar `org.gnome.Shell.Eval` antes da linha "GNOME Shell started" no log trava o arranque:
   esperar a linha antes da primeira chamada D-Bus.
9. Um `dbus-run-session` cria um segundo `dconf-service` que grava o mesmo `~/.config/dconf/user`
   do usuário; instâncias concorrentes sobrescrevem valores (ex.: `enabled-extensions` voltando)
   e desestabilizam o arranque. Isolar com `DCONF_PROFILE` apontando para um perfil
   `user-db:<nome>`; o nome vira elemento de caminho D-Bus, então **sem hífens**.
10. No modo headless o Xwayland às vezes não conecta ("could not connect to wayland server") e o
    mutter espera para sempre; testes que não precisam de X11 usam `--no-x11`.
11. A resposta de `gdbus call` é texto GVariant: `(true, '...')` não é literal Python.

## Contexto

Aceitação do gnome-furigana (RFC 0002) sem tocar na sessão gráfica do desenvolvedor.

## Metodo

VM QEMU/KVM headless (`-display none`), imagem cloud Ubuntu 24.04, `tests/shell/run-acceptance.sh`.

## Evidencia

Logs `gnome-shell.log` com "GNOME Shell started" e estado `ACTIVE` (1) da extensão via
`org.gnome.Shell.Extensions.ListExtensions`; ausência de "started" quando o arquivo de guarda existia.

## Interpretacao

Fato: com o arquivo de guarda presente as extensões não carregam. Inferência: o arranque lento sob
compilação pesada na mesma VM é contenção de CPU com renderização por software (llvmpipe).

## Consequencia operacional

O runner limpa o arquivo de guarda e sobras do compositor, isola o dconf, usa `--no-x11`, espera o
log de arranque antes de usar `Eval` e mata com SIGKILL um Shell travado.

## Riscos e limites

Renderização por software: medições de tempo de quadro não representam hardware real.
