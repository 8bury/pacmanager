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

## Intenção do aplicativo

O PacManager é uma loja open source para os pacotes do Arch Linux e do AUR:
uma interface simples e confiável sobre o pacman e o yay. Não é uma vitrine
comercial como as lojas da Apple ou da Microsoft, porque não tem parceiros,
patrocínio nem curadoria editorial.

- Nenhum aplicativo ganha mais destaque que os outros. Não crie banners,
  "em destaque", heróis, recomendações ou cartões maiores para um app.
  As seções mostram os mais usados e as categorias, com o mesmo peso visual
  para todos.
- A ordem vem de dados reais: popularidade do pkgstats e metadados AppStream.
- Não invente conteúdo que os repositórios não fornecem: avaliações, notas,
  capturas de tela, histórico de versões ou descrições.
- Operações acontecem no terminal, sempre com confirmação. O AUR fica sempre
  identificado como pacotes da comunidade sem revisão oficial.
- Polimento significa clareza, consistência, alinhamento e estados bem
  resolvidos (vazio, carregando, erro), e não ornamento. Ajustes secundários,
  como tema e filtros, ficam em menus que abrem ao clicar, não sempre visíveis.
- As referências visuais estão em `docs/design/references.md`. Use-as para
  o layout e a hierarquia, sem copiar o modelo comercial delas.
