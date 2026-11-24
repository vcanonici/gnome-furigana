# Matriz de compatibilidade — 0.1.0

Verificado em VM Ubuntu 24.04.5, GNOME Shell 46.0, gnome-control-center 1:46.7-0ubuntu0.24.04.6.

| Área | Estado | Observações |
|---|---|---|
| GNOME Shell inteiro | suportado | barra, menus, configurações rápidas, Overview, busca, notificações, diálogos: ruby acima quando cabe, senão `漢字(かんじ)` — nunca omitido; hover por palavra |
| Aplicativos GTK/gettext | suportado (inline) | Configurações, Arquivos, GTK, libadwaita…: catálogos `ja@furigana` (exige `sudo` na instalação); hover ainda não |
| Snap / Flatpak | não suportado | catálogos próprios dentro do pacote |
| Texto que não vem do gettext | não suportado | nomes de arquivos, conteúdo de documentos |
| Painel nas Configurações | experimental | exige o build correspondente à versão instalada do g-c-c |
| Outras versões do GNOME | não suportado | `shell-version: ["46"]` |
