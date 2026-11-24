# Indice de CLI tools

Liste aqui as CLI tools esperadas, disponiveis ou desejadas pelo projeto.

## Tools registradas

| Tool | Status | Comando | Quando usar | Registro |
| --- | --- | --- | --- | --- |
| gnome-furigana | disponível | `gnome-furigana --help` | Serviço D-Bus (`serve`), prévia de análise (`analyze`), `lookup`, `build-jmdict`, `build-catalogs` (catálogos `ja@furigana`: JSON com built/unchanged/empty/unsupported/failed; exit 1 se algum falhar; grava em `/usr/share/locale-langpack/ja@furigana` por padrão, só como root). | `service/src/main.rs` |
| install.sh | disponível | `./install.sh --help` | Instalar/atualizar por usuário a partir de release ou checkout. | `install.sh` |
| uninstall.sh | disponível | `./uninstall.sh --help` | Remover exatamente o que o manifesto registrou. | `uninstall.sh` |
| stage-release.sh | disponível | `packaging/stage-release.sh` | Montar `dist/` com binário, extensão, schema e unidades. | `packaging/stage-release.sh` |
| build-fork.sh | disponível | `settings-panel/build-fork.sh OUT` | Compilar as Configurações 46.7 com o painel Furigana (contêiner/VM). | `settings-panel/build-fork.sh` |
| run-acceptance.sh | disponível | `tests/shell/run-acceptance.sh [OUT]` | Aceitação em GNOME Shell headless — só em VM. | `tests/shell/run-acceptance.sh` |
