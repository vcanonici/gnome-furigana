# JMdict (snapshot versionado)

`JMdict_e.gz` é uma cópia do dicionário **JMdict** (somente glosas em inglês), mantida no
repositório para que a instalação seja autocontida: o release leva o índice já pronto e a
instalação não acessa a rede.

| Campo | Valor |
|---|---|
| Fonte | https://www.edrdg.org/pub/Nihongo/JMdict_e.gz |
| Gerado em | 2026-11-24 (`<!-- JMdict created: 2026-11-24 -->`) |
| SHA-256 | `8bac55a4fd3ea2b103348ced2098a61165a73aaea45754e6535057c71d7b14fa` |
| Licença | Creative Commons Attribution-ShareAlike 4.0 (CC BY-SA 4.0) |

Este pacote usa os arquivos do dicionário [JMdict/EDICT](https://www.edrdg.org/wiki/index.php/JMdict-EDICT_Dictionary_Project).
Esses arquivos são propriedade do [Electronic Dictionary Research and Development Group](https://www.edrdg.org/)
e são usados de acordo com a [licença do grupo](https://www.edrdg.org/edrdg/licence.html).

## Atualizar

```bash
curl -fsSL -o data/jmdict/JMdict_e.gz https://www.edrdg.org/pub/Nihongo/JMdict_e.gz
sha256sum data/jmdict/JMdict_e.gz   # atualizar a tabela acima
```

`packaging/stage-release.sh` gera `data/jmdict.sqlite` no release a partir deste arquivo e
confere o hash da tabela antes.
