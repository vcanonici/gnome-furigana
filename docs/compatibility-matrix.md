# Matriz de compatibilidade — 0.1.0

Verificado em VM Ubuntu 24.04.5, GNOME Shell 46.0, gnome-control-center 1:46.7-0ubuntu0.24.04.6.

| Área | Estado | Observações |
|---|---|---|
| Configurações rápidas | suportado | furigana + hover por palavra; menus expandidos são reescaneados enquanto abertos |
| Barra superior | suportado | pouco espaço acima: normalmente usa `漢字(かんじ)` ou omite |
| Overview / pesquisa | não suportado | planejado |
| Notificações | não suportado | desligado por privacidade |
| Diálogos do Shell | não suportado | |
| Aplicativos GTK | não suportado | planejado via AT-SPI (fase 5 da RFC) |
| Painel nas Configurações | experimental | exige o build correspondente à versão instalada do g-c-c |
| Outras versões do GNOME | não suportado | `shell-version: ["46"]` |
