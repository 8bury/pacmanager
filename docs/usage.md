# Guia de uso

## Uso

- Digite na busca para consultar pacotes oficiais, ou pressione Enter para buscar imediatamente. O botão de filtros dentro da barra permite incluir o AUR.
- Abra Explorar para conhecer aplicativos disponíveis nos repositórios configurados, ordenados pela popularidade do pkgstats quando disponível.
- Use Categorias para filtrar os aplicativos pelos metadados AppStream.
- Abra Instalados para ver aplicativos. Ative Mostrar componentes do sistema para ver também bibliotecas e ferramentas.
- Clique no aplicativo para abrir a página dele. Use Voltar para retornar à lista.
- Na página, instale, remova ou atualize o aplicativo quando houver uma atualização disponível.
- Abra Atualizações e use Atualizar repositórios para sincronizar os índices e consultar versões novas. Use Atualizar tudo para atualizar o sistema completo.

A busca mostra todos os resultados correspondentes, incluindo ferramentas de terminal. O filtro de componentes do sistema se aplica à lista de instalados. Pacotes instalados fora dos repositórios configurados aparecem como Origem externa; essa indicação não confirma que vieram do AUR.

Pacotes com o mesmo nome em várias origens aparecem em uma única linha. Clique nela ou na seta para expandir as opções de repositório, incluindo o AUR quando habilitado. A seta gira para baixo e as opções aparecem com uma animação. Cada opção mostra sua origem e versão, com ações para aquele pacote. Clique novamente na linha principal ou na seta para recolher as opções. A contagem da lista considera cada nome uma vez. Ao passar o mouse, toda a linha fica destacada, incluindo a área da ação.

Consultas são executadas em segundo plano. As operações abrem um terminal para autenticação, confirmação das alterações e revisão dos arquivos do AUR. Leia o resumo do gerenciador de pacotes antes de confirmar. O PacManager não armazena senhas nem aprova prompts automaticamente.

O catálogo usa [AppStream do Arch](https://archlinux.org/packages/extra/any/archlinux-appstream-data/) para identificar aplicativos, nomes, descrições e categorias. A seção de sugestões na página de um aplicativo mostra outros aplicativos que compartilham suas categorias. A popularidade vem da [API pública do pkgstats](https://pkgstats.archlinux.de/api/doc), baseada nos relatos voluntários recebidos pelo serviço. O PacManager consulta essas estatísticas sem enviar seus pacotes instalados.

Se houver um catálogo AppStream local, ele será usado. Caso contrário, o PacManager baixa os metadados dos [servidores oficiais do Arch](https://sources.archlinux.org/other/packages/archlinux-appstream-data/), sem instalar pacotes. Os dados processados ficam em `$XDG_CACHE_HOME/pacmanager`, ou `~/.cache/pacmanager`, por 24 horas. Se uma atualização falhar, o cache anterior continua disponível. Sem dados de popularidade, os aplicativos aparecem em ordem alfabética. A busca no AUR continua disponível; as sugestões usam somente o catálogo AppStream dos repositórios oficiais.

A busca ordena os resultados por relevância. O nome exato do pacote aparece primeiro, seguido de correspondências no nome antes de correspondências apenas na descrição. Entre resultados semelhantes, aplicativos principais têm preferência sobre componentes e variantes como `-git` e `-bin`; a popularidade do pkgstats ajuda a desempatar. Nos resultados do AUR, os campos de popularidade e votos também são usados, sem misturar suas escalas com as porcentagens do pkgstats. A busca reaproveita o cache do catálogo e não faz consultas adicionais de popularidade a cada pesquisa. Sem essas estatísticas, mantém a ordenação por relevância e usa o nome como desempate.

## Requisitos

Linux com pacman, um terminal compatível e sudo configurado para operações oficiais. O yay é necessário para o AUR. Execute a interface como seu usuário normal.

A lista de atualizações depende das bases locais do pacman. Atualizar repositórios abre um terminal e executa `sudo pacman -Sy`; após o sucesso, a lista é recarregada automaticamente. Atualizar tudo sincroniza as bases e atualiza o sistema. Após sincronizar os repositórios, prefira Atualizar tudo antes de atualizar pacotes individuais para evitar atualizações parciais. Na aba Instalados, Recarregar consulta novamente as bases locais.

Atualizações individuais dos repositórios usam `pacman -S --needed repositório/pacote`, sem sincronizar as bases e sem atualizar todos os pacotes. Dependências necessárias também podem mudar. O AUR usa `yay -S --aur --needed pacote`. A confirmação explica que atualizações parciais podem causar incompatibilidades; o Arch recomenda a atualização completa do sistema. [Orientação do Arch](https://wiki.archlinux.org/title/System_maintenance#Partial_upgrades_are_unsupported).

Os ícones vêm das entradas de desktop e dos temas locais, incluindo arquivos PNG e SVG. Aplicativos sem ícone disponível recebem um símbolo genérico. Nenhum ícone é baixado da internet.

A classificação de aplicativos usa entradas de desktop e regras auxiliares. Ela pode omitir ferramentas de terminal ou programas sem metadados. Mostrar componentes do sistema permite encontrar essas exceções. Ocultar um pacote não o protege contra remoção nem o exclui das atualizações.

## Desenvolvimento

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
target/debug/pacmanager --check
```

O backend e os modelos ficam em `src/backend.rs` e `src/model.rs`. A interface fica em `src/app.rs`. Os testes usam dados controlados e não instalam, removem ou atualizam pacotes reais.

`--check` consulta o sistema real sem executar transações. A busca no AUR tem limite de 20 segundos, e consultas a processos têm limite de 30 segundos. Se a comunidade estiver indisponível, a interface mantém os resultados dos repositórios configurados e exibe o erro.

O teste de integração com o AUR real é opcional e exige pacman e acesso à rede:

```sh
cargo test --test aur_search -- --ignored
```

Para adicionar ao menu do desktop depois de compilar, execute `./install.sh` na raiz do projeto. O instalador copia o binário, a entrada de desktop e os ícones do PacMan para os diretórios do seu usuário.
