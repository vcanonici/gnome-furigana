# Changelog

## 0.1.0 — experimental

- Extensão GNOME Shell 46 (TypeScript): furigana acima do kanji nas configurações rápidas e na
  barra superior, com política de ajuste (escala mínima → `漢字(かんじ)` → omitir).
- Tradução pt-BR após 2 s parado sobre a mesma palavra; cartão com leitura, sentidos e origem.
- Toggle "ふりがな" nas configurações rápidas.
- Serviço linguístico em Rust (Lindera + IPADIC embutido), D-Bus `io.github.vcanonici.Furigana1`,
  glossário pt-BR curado a partir do vocabulário real do Shell 46, JMdict (inglês) como fallback.
- Painel "Furigana" nas Configurações (build do gnome-control-center 46.7 do Ubuntu, por usuário).
- `install.sh` / `uninstall.sh` por usuário, com manifesto e restauração do idioma.
