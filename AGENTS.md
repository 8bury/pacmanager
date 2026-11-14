# Guia para agentes

- Nunca abra janelas na sessão gráfica do usuário: elas aparecem e roubam o
  foco enquanto ele usa o computador. Para capturas use
  `bash scripts/demo-shot.sh .cache/NOME.png [args da demo]`; para qualquer
  outro comando com janela use `bash scripts/headless.sh COMANDO`.
  `cargo run` com janela fica para quando o usuário pedir.
- Use o modo `--demo` para revisar a interface; ele não consulta nem altera
  pacotes. Capturas ficam em `.cache/`, fora do Git.
- Antes de concluir mudanças de código: `cargo fmt --check`,
  `cargo clippy --locked --all-targets -- -D warnings` e `cargo test --locked`.
- Mudanças na janela ou no OpenGL: rode também
  `python3 scripts/test-wayland-focus.py`.
