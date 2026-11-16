# Changelog

## 0.2.1 · 2026-11-16

- Atualizar tudo e as operações do AUR pedem a senha do sudo só no início e a mantêm válida durante a compilação dos pacotes, sem um segundo pedido que expirava enquanto ninguém olhava o terminal.

## 0.2.0 · 2026-11-14

- Cabeçalhos de página com título grande, contagem e ações alinhadas à direita.
- Seções do Explorar separadas por espaço, sem divisórias.
- Ícones genéricos com a inicial do aplicativo sobre uma cor própria.
- Confirmação centralizada com fundo escurecido, ação nomeada e remoção em vermelho.
- Página do aplicativo sem descrição repetida, com metadados em cartão e avisos destacados.
- Chips de categoria com opção Todos, interruptor de pacotes do sistema e botões consistentes.
- Estados vazios ilustrados, avisos de erro em faixa e barra de status dispensável.
- Barra lateral com nome do app, busca com foco visível, contador em cápsula,
  ícone próprio para Atualizações e tema escolhido num menu de Aparência.
- Filtro da busca alinhado ao campo, com indicação quando o AUR está incluído.
- Atalhos Ctrl+F para buscar e Esc para voltar da página do aplicativo.
- Animações suaves de hover em botões, navegação e destaque.
- Opções `--demo-search` e `--demo-confirm` para revisar mais telas no modo demo.

## 0.1.1 · 2026-10-05

- Ícone do PacMan no menu do desktop e na janela do aplicativo.
- Identificador da janela alinhado à entrada de desktop em Wayland e X11.
- Instalador para o usuário, incluindo ícones e caminho absoluto do executável.
- Screenshots refeitas com a interface atual.
- Script para empacotar a release com os recursos de instalação.

## 0.1.0 · 2026-10-05

Primeira versão pública do PacManager.

- Interface gráfica para pacotes do Arch Linux e derivados, escrita em Rust com egui/eframe.
- Catálogo AppStream, categorias e ordenação pela popularidade do pkgstats.
- Busca nos repositórios configurados e no AUR.
- Listas de instalados e atualizações, com páginas de detalhes dos aplicativos.
- Instalação, remoção e atualização pelo terminal, preservando autenticação e confirmação.
- Temas claro, escuro e do sistema, com preferência salva.
- Modo de demonstração e verificação do backend sem transações.
- Binário para Linux x86_64, entrada de desktop e checksums SHA-256.
