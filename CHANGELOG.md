# Changelog

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
