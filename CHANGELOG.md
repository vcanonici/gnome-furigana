# Changelog

## 0.2.0 — furigana em todo o sistema

- **Apps GTK/gettext** (Configurações, Arquivos, GTK, libadwaita…): catálogos `ja@furigana`
  gerados do sistema (`gnome-furigana build-catalogs`), só com as mensagens anotadas; markup,
  placeholders e mnemônicos preservados; regenerados após cada `apt` (gancho instalado com sudo).
- **GNOME Shell inteiro**: barra, menus, Overview, busca, notificações e diálogos; regra nova:
  furigana acima quando há espaço, senão `漢字(かんじ)` — nunca omitido.
- **Leituras melhores**: UniDic (NINJAL) no lugar do IPADIC (行 ぎょう, 正しく ただしく, 値 あたい…);
  leituras revisadas do glossário têm prioridade; datas e contadores (11月 じゅういちがつ, 24日 にじゅうよっか).
- **Instalação autocontida**: JMdict versionado no repo e índice pronto no release.
- **Desempenho**: o Shell ocioso ficava em ~50–100% de um núcleo (um later `BEFORE_REDRAW`
  persistente forçava um frame atrás do outro). Agora a sincronização roda no `before-update` do
  stage, escreve só o que mudou e a varredura de rótulos é fatiada: ~0% ocioso.
- Cartão de tradução de datas: `25日` → "dia 25", `11月` → "novembro".
- Instalador: `LANGUAGE=ja@furigana` via `~/.pam_environment`, `--settings-follow-session`,
  `--no-system`; remove a extensão de `disabled-extensions`; backups com restauração exata.

## 0.1.0 — experimental

- Extensão GNOME Shell 46 (TypeScript): furigana acima do kanji nas configurações rápidas e na
  barra superior, com política de ajuste (escala mínima → `漢字(かんじ)` → omitir).
- Tradução pt-BR após 2 s parado sobre a mesma palavra; cartão com leitura, sentidos e origem.
- Toggle "ふりがな" nas configurações rápidas.
- Serviço linguístico em Rust (Lindera + IPADIC embutido), D-Bus `io.github.vcanonici.Furigana1`,
  glossário pt-BR curado a partir do vocabulário real do Shell 46, JMdict (inglês) como fallback.
- Painel "Furigana" nas Configurações (build do gnome-control-center 46.7 do Ubuntu, por usuário).
- `install.sh` / `uninstall.sh` por usuário, com manifesto e restauração do idioma.
