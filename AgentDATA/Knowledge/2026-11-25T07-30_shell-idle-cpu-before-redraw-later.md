# Shell ocioso a 50–100% de CPU: later BEFORE_REDRAW persistente

- **Sintoma:** com a extensão ativa, `gnome-shell` ocioso usava ~50% de um núcleo no PC
  (5% sem a extensão) e ~100% na VM (renderização por software); CPU a 90–97 °C.
- **Causa:** `global.compositor.get_laters().add(Meta.LaterType.BEFORE_REDRAW, …)` retornando
  `SOURCE_CONTINUE`: enquanto há later pendente, o mutter agenda outro frame, então a tela
  é redesenhada continuamente. Reduzir o trabalho dentro do callback não resolve.
- **Correção:** `global.stage.connect('before-update', …)`, que só roda em frames que já iam
  acontecer; escrever posição/opacidade/visibilidade só quando mudam; elevar a camada só em
  `child-added` do `uiGroup`; varredura de rótulos fatiada (`LabelScanner`).
- **Medição:** VM com GDM: ~100% → ~0% ocioso; aceitação 11/11 PASS.
- **Lição:** medir o `gnome-shell` ocioso (top em regime estável, > 30 s após o login) na
  VM antes de liberar mudanças em hooks por frame.
- **Relacionado:** `forget()` de rótulo destruído não pode reescrever `label.text`
  (Clutter-CRITICAL `CLUTTER_IS_TEXT`); usar `AnnotatedLabel.detach()`.
