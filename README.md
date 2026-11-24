# gnome-furigana

Transforma a interface japonesa do GNOME num ambiente de estudo:

- **furigana em todo o GNOME Shell** (barra, menus, Overview, notificações…): acima do kanji quando
  há espaço, senão `漢字(かんじ)` — **nunca omitido**;
- **furigana nos apps GTK** (Configurações, Arquivos…), inline, por catálogos de tradução anotados;
- **tradução pt-BR de uma palavra** ao deixar o mouse parado sobre ela por 2 segundos (Shell);
- **painel "Furigana" dentro das Configurações** do GNOME.

Alvo: **Ubuntu 24.04 · GNOME Shell 46 · Wayland · x86_64**. Estado: **experimental (0.1.0)**.

## Instalar

```bash
curl -fsSL https://raw.githubusercontent.com/vcanonici/gnome-furigana/main/install.sh | bash
```

ou, a partir do código:

```bash
git clone https://github.com/vcanonici/gnome-furigana && cd gnome-furigana
./install.sh --source .
```

O instalador roda como usuário e pede `sudo` uma vez para gerar os catálogos `ja@furigana`
(em `/usr/share/locale-langpack/ja@furigana`, regenerados automaticamente após cada `apt`) e para
pacotes apt ausentes. Os catálogos originais **nunca** são alterados. Ele pergunta se deve mudar o
idioma da sessão para japonês. Tudo passa a funcionar **no próximo login**. A instalação é
autocontida: nada é baixado além do próprio release (nada com `--source`).

| Opção | Efeito |
|---|---|
| `--yes` | sem perguntas; mantém o idioma |
| `--language ja\|keep` | troca a sessão para japonês ou mantém |
| `--no-jmdict` | não baixa o JMdict (traduções só do glossário curado) |
| `--no-settings-panel` | não instala o painel nas Configurações |
| `--no-apt` | não instala pacotes apt |
| `--no-system` | sem sudo: só o Shell (apps GTK ficam sem furigana) |
| `--settings-follow-session` | remove de um atalho seu das Configurações o forçar de idioma |
| `--version X.Y.Z` / `--source DIR` | versão do release / diretório local |

Desinstalar: `~/.local/share/gnome-furigana/…` é removido por `./uninstall.sh` (lê o manifesto do
instalador, desativa a extensão e oferece restaurar o idioma anterior).

## Como funciona

```text
GNOME Shell (extensão TypeScript → GJS)           Serviço linguístico (Rust, ~17 MB RSS)
  descobre St.Label → hit test por palavra   D-Bus   Lindera + UniDic embutido (leituras)
  furigana em atores sobrepostos, não-reativos ◄──►  alinhamento kanji/okurigana
  hover 2000 ms → cartão pt-BR                       glossário pt-BR curado → JMdict (en)
Configurações (fork do g-c-c 46.7) ── GSettings org.gnome.shell.extensions.furigana ──┘
Apps GTK ── LANGUAGE=ja@furigana:ja ── catálogos gerados pelo mesmo serviço (build-catalogs)
```

- O rótulo original **nunca é substituído**: o furigana é desenhado em atores separados que não
  recebem eventos, então cliques, foco e leitores de tela continuam iguais.
- O serviço é ativado sob demanda pelo D-Bus, roda numa unidade `systemd --user` sem acesso à rede
  (`RestrictAddressFamilies=AF_UNIX`) e **nunca grava o texto analisado em disco**.
- Significados: seu vocabulário (`~/.config/gnome-furigana/overrides.tsv`) → glossário curado de UI
  em pt-BR → JMdict (inglês, marcado como tal). Sem fonte confiável: **"Tradução não disponível"**.

### O painel nas Configurações

O `gnome-control-center` não aceita plugins, então o release inclui um binário das Configurações
46.7 compilado a partir do **pacote-fonte do próprio Ubuntu** com um painel a mais. Ele é instalado
em `~/.local/share/gnome-furigana/settings` e ativado só para o seu usuário por *overrides* de
`org.gnome.Settings.desktop` e do serviço D-Bus — o pacote do sistema fica intacto. Quando o Ubuntu
atualizar o `gnome-control-center`, rode o instalador de novo para obter um build correspondente
(ou `--no-settings-panel`; as mesmas opções existem em *Extensões → Furigana*).

## O que é suportado

| Componente | Estado |
|---|---|
| GNOME Shell inteiro | ruby ou inline (nunca omitido) + hover por palavra |
| Apps GTK/gettext (Configurações, Arquivos…) | inline `漢字(かんじ)`; hover ainda não |
| Snap/Flatpak, texto fora do gettext | ainda não |

Leituras usam análise morfológica com contexto (UniDic, NINJAL) e um glossário revisado que tem
prioridade; nomes próprios e palavras ambíguas podem sair sem leitura em vez de errados. Veja [docs/compatibility-matrix.md](docs/compatibility-matrix.md).

## Desenvolvimento

Requer Node ≥ 22.23.1, Rust (via rustup; versão fixada em `service/rust-toolchain.toml`),
`shellcheck`, `glib-compile-schemas` e `zip`.

```bash
npm ci
npm run check                 # lint + testes + build (Nx, igual à CI)
packaging/stage-release.sh    # dist/gnome-furigana-<versão>-x86_64.tar.gz
```

Testes de aceitação num GNOME Shell 46 headless (apenas em VM descartável):
`tests/shell/run-acceptance.sh`. Detalhes em [docs/development.md](docs/development.md) e o
contrato D-Bus em [docs/protocol-dbus.md](docs/protocol-dbus.md).

## Licenças

Código: GPL-2.0-or-later. Dicionário morfológico UniDic (NINJAL) embutido via Lindera.
JMdict © EDRDG, CC BY-SA 4.0, versionado em `data/jmdict/` (ver o README de lá). Glossário pt-BR:
parte deste projeto.
