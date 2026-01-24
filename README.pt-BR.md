<p align="center">
  <a href="README.md">English</a> |
  <a href="README.pt-BR.md">Português</a>
</p>

<div align="center">

# KoShelf

<p>
  <a href="https://github.com/zanivann/KoShelf/stargazers">
    <img src="https://img.shields.io/github/stars/zanivann/koshelf?style=social" alt="Stars" />
  </a>
  <a href="https://github.com/zanivann/KoShelf/tags">
    <img src="https://img.shields.io/github/v/release/zanivann/koshelf?label=release" alt="Latest Release" />
  </a>
  <a href="https://github.com/zanivann/KoShelf/blob/main/License">
    <img src="https://img.shields.io/github/license/zanivann/koshelf" alt="License" />
  </a>
</p>

![Painel de estatísticas](https://github.com/user-attachments/assets/94a094d2-298b-412c-80b3-b3b2e2cfc6de)

###### Uma ferramenta CLI em Rust que gera um belo site estático a partir da sua biblioteca do KOReader, exibindo sua coleção de ebooks com destaques, anotações e progresso de leitura.

</div>

---

## Sumário

- [Funcionalidades](#funcionalidades)
- [Capturas de Tela](#capturas-de-tela)
- [Instalação](#instalação)
  - [Home Assistant](#home-assistant)
  - [Binários Pré-compilados](#binários-pré-compilados)
  - [A partir do Código-Fonte](#a-partir-do-código-fonte)
- [Uso](#uso)
  - [Uso Básico](#uso-básico)
  - [Modos de Operação](#modos-de-operação)
  - [Opções de Linha de Comando](#opções-de-linha-de-comando)
  - [Exemplo](#exemplo)
- [Configuração do KOReader](#configuração-do-koreader)
  - [Opções de Armazenamento de Metadados](#opções-de-armazenamento-de-metadados)
  - [Configuração Típica de Implantação](#configuração-típica-de-implantação)
- [Dados Suportados](#dados-suportados)
  - [A partir de Arquivos EPUB](#a-partir-de-arquivos-epub)
  - [A partir de Metadados do KOReader](#a-partir-de-metadados-do-koreader)
  - [A partir do Banco de Estatísticas do KOReader](#a-partir-do-banco-de-estatísticas-do-koreader-statisticssqlite3)
- [Estrutura do Site Gerado](#estrutura-do-site-gerado)
- [Créditos](#créditos)
- [Aviso Legal](#aviso-legal)

---

## Funcionalidades

- 📚 **Visão Geral da Biblioteca (Livros + Quadrinhos)**: Exibe itens em leitura, concluídos e não lidos (ebooks e quadrinhos)
- 🎨 **UI Moderna**: Design elegante com Tailwind CSS, tipografia limpa e layout responsivo
- 📝 **Anotações, Destaques e Avaliações**: Todos os destaques, notas, avaliações em estrelas e notas de resumo do KOReader exibidos na página de detalhes de cada livro
- 📊 **Estatísticas de Leitura**: Acompanhe seus hábitos com tempo de leitura, páginas lidas, mapas de calor configuráveis e resumo semanal
- 📅 **Calendário de Leitura**: Visão mensal com atividade diária e estatísticas por mês
- 🎉 **Retrospectiva Anual**: Linha do tempo de leituras concluídas, resumos mensais e detalhes ricos por livro
- 📈 **Estatísticas por Livro**: Sessões, duração média, velocidade de leitura e última data de leitura
- 🔍 **Busca e Filtros**: Pesquisa por título, autor ou série, com filtros por status de leitura
- 🚀 **Site Estático**: Gera um site completo que pode ser hospedado em qualquer lugar
- 🖥️ **Modo Servidor**: Servidor web integrado com monitoramento de arquivos
- 📱 **Responsivo**: Otimizado para desktop, tablet e mobile
- 🔌 **API**: Endpoint REST para obter estatísticas da biblioteca

---

## Capturas de Tela

![Visão geral da biblioteca](https://github.com/user-attachments/assets/ad096bc9-c53a-40eb-9de9-06085e854a26)
![Detalhes do livro](https://github.com/user-attachments/assets/44113be0-aa19-4018-b864-135ddb067a9d)
![Calendário de leitura](https://github.com/user-attachments/assets/a4ac51f1-927e-463d-b2d6-72c29fdc4323)
![Retrospectiva](https://github.com/user-attachments/assets/9558eea9-dee1-4b0a-adac-1bc0157f0181)

---

## Instalação

### Implantação com Docker Compose

Implante o KoShelf facilmente usando a imagem Docker mantida pela comunidade.

#### Início Rápido

1. Crie um arquivo `docker-compose.yml`:

```yaml
services:
  koshelf:
    image: ghcr.io/devtigro/koshelf:latest
    ports:
     - "3009:3009"
    volumes:
      - /path/to/your/books:/books:ro
      - /path/to/your/settings:/settings:ro
    restart: unless-stopped
    ```

## Atualização dos caminhos de volume

2. Atualize os caminhos dos volumes:
- Substitua `/path/to/your/books` pelo **caminho absoluto da sua biblioteca de livros**
- Substitua `/path/to/your/settings` pelo **caminho absoluto do diretório de configurações**

3. Inicie o container:
```bash
docker compose up -d
```

4. Acesse o KoShelf em http://localhost:3009

Repositório da Imagem Docker: [koshelf-docker](https://github.com/DevTigro/koshelf-docker)

### Instalação (CasaOS)

1. Abra o CasaOS > App Store > Custom Install.
2. Clique no ícone **Import** (canto superior direito).
3. Cole a seguinte configuração YAML:
```yaml
name: koshelf-zanivann
services:
  koshelf:
    container_name: koshelf-zanivann
    image: ghcr.io/zanivann/koshelf:latest
    cpu_shares: 90
    command:
      - --library-path
      - /books
      - --statistics-db
      - /settings/statistics.sqlite3
      - --port
      - "3009"
      - --timezone
      - America/Sao_Paulo
      - --include-unread
      - --language
      - pt_BR
    ports:
      - target: 3009
        published: "3009"
        protocol: tcp
    restart: unless-stopped
    volumes:
      - type: bind
        source: /DATA/AppData/koshelf-books
        target: /books
      - type: bind
        source: /DATA/AppData/koshelf
        target: /settings
    x-casaos:
      icon: [https://b.thumbs.redditmedia.com/Flac-ySmslzY0SE583PNA42rFbcYxLt7hqgCeUrC11s.png](https://b.thumbs.redditmedia.com/Flac-ySmslzY0SE583PNA42rFbcYxLt7hqgCeUrC11s.png)
      title: KoShelf Personalizado
    network_mode: bridge
x-casaos:
  author: zanivann
  category: self
  icon: [https://b.thumbs.redditmedia.com/Flac-ySmslzY0SE583PNA42rFbcYxLt7hqgCeUrC11s.png](https://b.thumbs.redditmedia.com/Flac-ySmslzY0SE583PNA42rFbcYxLt7hqgCeUrC11s.png)
  index: /
  port_map: "3009"
  scheme: http
  title:
    custom: KoShelf Personalizado
```
### Binários Pré-compilados

A maneira mais fácil de começar é baixar um binário pré-compilado na [página de releases](https://github.com/zanivann/koshelf/releases). Os binários estão disponíveis para:

- Windows (x64)
- macOS (Apple Silicon, Intel e Universal)
- Linux (x64 e ARM64)

Observe que o KoShelf é uma ferramenta de linha de comando, portanto você precisará executá-lo a partir de um terminal (macOS/Linux) ou PowerShell/Prompt de Comando no Windows. Dar duplo clique no executável não funcionará, pois ele exige argumentos de linha de comando para funcionar corretamente.

**Nota para usuários do Windows**: o Windows Defender provavelmente marcará e excluirá o binário do Windows como se fosse um vírus (mais informações [aqui](https://medium.com/opsops/is-windows-broken-7f8de8b8f3ad)). Isso é um falso positivo se você baixou o binário diretamente deste repositório. Para utilizá-lo:

1. Restaure o arquivo no histórico de proteção do Windows Defender (Segurança do Windows > Proteção contra vírus e ameaças > Histórico de proteção > Restaurar)
2. Execute o binário pelo PowerShell ou Windows Terminal com argumentos — dar duplo clique fará com que ele feche imediatamente, pois nenhum argumento é fornecido

#### Primeira vez usando linha de comando?

Se você nunca usou a linha de comando antes, veja como começar:

**Windows:**
1. Pressione `Win + R`, digite `powershell` e pressione Enter
2. Navegue até o local onde você baixou o binário do KoShelf (ex.: `cd C:\Users\SeuNome\Downloads`)
3. Execute a ferramenta com os argumentos desejados (veja os exemplos abaixo)

**macOS e Linux:**
1. Pressione `Cmd + Espaço`, digite `terminal` e pressione Enter
2. Navegue até o local onde você baixou o binário do KoShelf (ex.: `cd ~/Downloads`)
3. Torne o arquivo executável: `chmod +x koshelf` (normalmente não é necessário no macOS, pois o binário é assinado)
4. Execute a ferramenta com os argumentos desejados (veja os exemplos abaixo)

**Exemplo:**
```bash
# Generate site from a library folder
./koshelf -i ~/Library -o ~/my-reading-site -t "My Reading Journey"

# Generate site from multiple folders (e.g., books + comics)
./koshelf -i ~/Books -i ~/Comics -o ~/my-reading-site

# Generate site with statistics and unread items included
./koshelf -i ~/Library -o ~/my-reading-site --statistics-db ~/KOReaderSettings/statistics.sqlite3 --include-unread

# Start web server with live file watching and statistics
./koshelf -i ~/Library -s ~/KOReaderSettings/statistics.sqlite3 -p 8080

# Generate static site with file watching and statistics
./koshelf --library-path ~/Library -o ~/my-reading-site --statistics-db ~/KOReaderSettings/statistics.sqlite3 --watch

# Generate site with custom heatmap color scaling (2 hours = highest intensity)
./koshelf -i ~/Library -s ~/KOReaderSettings/statistics.sqlite3 -o ~/my-reading-site --heatmap-scale-max 2h

# Generate site with custom heatmap color scaling (1.5 hours = highest intensity)
./koshelf -i ~/Library -s ~/KOReaderSettings/statistics.sqlite3 -o ~/my-reading-site --heatmap-scale-max 1h30m

# Generate site with explicit timezone and non-midnight day start (good for night owls)
./koshelf -i ~/Library -s ~/KOReaderSettings/statistics.sqlite3 -o ~/my-reading-site --timezone Australia/Sydney --day-start-time 03:00

# Using hashdocsettings (metadata stored by content hash)
./koshelf -i ~/Books -o ~/my-reading-site --hashdocsettings-path ~/KOReaderSettings/hashdocsettings

# Using docsettings (metadata stored in central folder by path)
./koshelf -i ~/Books -o ~/my-reading-site --docsettings-path ~/KOReaderSettings/docsettings

# Generate site with German UI language
./koshelf -i ~/Library -o ~/my-reading-site --language de_DE
```

## Configuração do KOReader

### Opções de Armazenamento de Metadados

O KOReader oferece três formas de armazenar metadados dos livros (progresso de leitura, destaques, anotações). O KoShelf é compatível com todas elas:

#### 1. Padrão: Metadados ao Lado dos Livros (Recomendado)

Por padrão, o KOReader cria pastas `.sdr` ao lado de cada arquivo de livro:

```
Books/
├── Book Title.epub
├── Book Title.sdr/
│   └── metadata.epub.lua
├── Another Book.epub
├── Another Book.sdr/
│   └── metadata.epub.lua
└── ...
```

Essa é a configuração mais simples — basta apontar `--library-path` para a pasta dos seus livros.

#### 2. Hashdocsettings

Se você selecionar **"hashdocsettings"** nas configurações do KOReader, os metadados são armazenados em uma pasta central organizada por hash do conteúdo:

**Uso:**
```bash
./koshelf --library-path ~/Books --hashdocsettings-path ~/KOReaderSettings/hashdocsettings
```

#### 3. Docsettings

Se você selecionar **"docsettings"** nas configurações do KOReader, o KOReader espelha a estrutura de pastas dos seus livros em uma pasta central e armazena os metadados nesse local:

```
KOReaderSettings/
└── docsettings/
    └── home/
        └── user/
            └── Books/
                ├── Book Title.sdr/
                │   └── metadata.epub.lua
                └── Another Book.sdr/
                    └── metadata.epub.lua
```

**Observação:** Diferente do KOReader, o KoShelf associa os livros apenas pelo nome do arquivo, pois a estrutura de pastas reflete o caminho do dispositivo (que pode ser diferente do caminho local). Se você tiver vários livros com o mesmo nome de arquivo, o KoShelf exibirá um erro — nesse caso, utilize `hashdocsettings` ou **metadados ao lado dos livros**.

**Uso:**
```bash
./koshelf --library-path ~/Books --docsettings-path ~/KOReaderSettings/docsettings
```

### Configuração Típica de Implantação

Embora existam muitas formas de usar esta ferramenta, abaixo está a forma como eu a utilizo:

1. **Sincronização com Syncthing**: utilizo o [Syncthing](https://syncthing.net/) para sincronizar tanto a pasta de livros quanto a pasta de configurações do KOReader do meu e-reader para o servidor
2. **Livros e Estatísticas**: aponto para a pasta sincronizada de livros usando `--books-path` e para o arquivo `statistics.sqlite3` dentro da pasta sincronizada de configurações do KOReader usando `--statistics-db`
3. **Modo Servidor Web**: executo o KoShelf em modo servidor web (sem `--output`) — ele reconstrói automaticamente quando os arquivos são alterados
4. **Proxy Reverso Nginx**: utilizo um proxy reverso nginx para HTTPS e para restringir o acesso

Dessa forma, sempre que o Syncthing sincroniza atualizações do meu e-reader, o site é atualizado automaticamente com meu progresso de leitura mais recente, novos destaques e estatísticas atualizadas.

### Configurações Contribuídas pela Comunidade

Veja [Configurações com Syncthing](docs/syncthing_setups/README.md) para guias criados pela comunidade sobre como sincronizar seus dispositivos com o KoShelf.

## Dados Suportados

### Formatos Suportados
- ePUB
- fb2 / fb2.zip
- mobi (não criptografado)
- CBZ
- CBR (não suportado no Windows — use a build Linux via [WSL](https://learn.microsoft.com/de-de/windows/wsl/install) se precisar)

### A partir de Arquivos EPUB
- Título do livro
- Autores
- Descrição (HTML sanitizado)
- Imagem de capa
- Idioma
- Editora
- Informações de série (nome e número)
- Identificadores (ISBN, ASIN, Goodreads, DOI, etc.)
- Assuntos/Gêneros

### A partir de Arquivos FB2
- Título do livro
- Autores
- Descrição (HTML sanitizado)
- Imagem de capa
- Idioma
- Editora
- Informações de série (nome e número)
- Identificadores (ISBN)
- Assuntos/Gêneros

### A partir de Arquivos MOBI (não criptografados)
- Título do livro
- Autores
- Descrição
- Imagem de capa
- Idioma
- Editora
- Identificadores (ISBN, ASIN)
- Assuntos/Gêneros

### A partir de Arquivos de Quadrinhos (CBZ/CBR)
Observação: **As builds para Windows suportam apenas CBZ** (CBR/RAR não é suportado).
- Título do livro (a partir de metadados ou do nome do arquivo)
- Informações de série (Série e Número)
- Autores (roteiristas, artistas, editores etc.)
- Descrição (Resumo)
- Editora
- Idioma
- Gêneros
- Imagem de capa (primeira imagem do arquivo)

### A partir dos Metadados do KOReader
- Status de leitura (lendo/concluído)
- Destaques e anotações com informações de capítulo
- Notas associadas aos destaques
- Percentual de progresso de leitura
- Avaliação (estrelas de 0 a 5)
- Nota de resumo (preenchida ao final do livro)

### A partir do Banco de Dados de Estatísticas do KOReader (statistics.sqlite3)
- Tempo total de leitura e páginas
- Estatísticas semanais de leitura
- Mapa de calor de atividade de leitura com escala personalizável (automática ou máximo fixo)
- Sessões e estatísticas de leitura por livro
- Cálculo de velocidade de leitura
- Rastreamento da duração das sessões
- Conclusões de livros (usadas na Retrospectiva Anual)

## Estrutura do Site Gerado
```
site/
├── index.html              # Main library page (books list if any books exist; otherwise comics list)
├── manifest.json           # PWA Manifest
├── service-worker.js       # PWA Service Worker
├── cache-manifest.json     # PWA Smart Cache Manifest
├── version.txt             # Version timestamp for lightweight polling
├── recap/                  # Yearly Recap pages
│   ├── index.html          # Empty state / Recap landing
│   ├── 2024/
│   │   ├── index.html      # Yearly recap page
│   │   ├── books/
│   │   │   └── index.html  # Yearly recap page (books only; only generated when both books+comics exist)
│   │   └── comics/
│   │       └── index.html  # Yearly recap page (comics only; only generated when both books+comics exist)
│   └── ...
├── statistics/
│   ├── index.html          # Reading statistics dashboard
│   ├── books/
│   │   └── index.html      # Reading statistics dashboard (books only; only generated when both books+comics exist)
│   └── comics/
│       └── index.html      # Reading statistics dashboard (comics only; only generated when both books+comics exist)
├── calendar/
│   └── index.html          # Reading calendar view
├── books/                  # Individual book pages
│   ├── list.json           # Manifest of all books (convenience only; not used by frontend)
│   ├── book-id1/           
│   │   ├── index.html      # Book detail page with annotations
│   │   ├── details.md      # Markdown export (human-readable)
│   │   └── details.json    # JSON export (machine-readable)
│   └── ...
├── comics/                 # Comics list + individual comic pages
│   ├── index.html          # Comics list page (only when books also exist; otherwise list is at /index.html)
│   ├── list.json           # Manifest of all comics (convenience only; not used by frontend)
│   ├── comic-id1/
│   │   ├── index.html      # Comic detail page with annotations
│   │   ├── details.md      # Markdown export (human-readable)
│   │   └── details.json    # JSON export (machine-readable)
│   └── ...
└── assets/
    ├── covers/             # Optimized cover images
    │   ├── book-id1.webp
    │   └── ...
    ├── recap/              # Social media share images (generated per year)
    │   ├── 2024_share_story.webp
    │   ├── 2024_share_story.svg
    │   ├── 2024_share_square.webp
    │   ├── 2024_share_square.svg
    │   ├── 2024_share_banner.webp
    │   └── 2024_share_banner.svg
    ├── css/
    │   ├── style.css       # Compiled Tailwind CSS
    │   └── event-calendar.min.css # Event calendar library styles
    ├── js/
    │   ├── library_list.js # Search and filtering functionality
    │   ├── item_detail.js  # Item detail page functionality
    │   ├── lazy-loading.js # Image lazy loading
    │   ├── section-toggle.js # Section collapsing/expanding
    │   ├── statistics.js   # Statistics page functionality
    │   ├── heatmap.js      # Activity heatmap visualization
    │   ├── calendar.js     # Calendar initialization
    │   ├── event-calendar.min.js # Event calendar library
    │   ├── recap.js        # Recap page interactions
    │   ├── storage-manager.js # Centralized local storage management
    │   └── pwa.js          # PWA installation and update logic
    ├── icons/              # PWA icons
    │   ├── icon-192.png
    │   └── icon-512.png
    └── json/               # Data files used by the frontend (for dynamic loading)
        ├── locales.json        # UI translations for the selected language
        ├── calendar/           # Calendar data split by month
        │   ├── available_months.json 
        │   ├── 2024-01.json   
        │   └── ...            
        └── statistics/         # Statistics data
            ├── all/            # Always generated when stats are enabled
            │   ├── week_0.json
            │   ├── ...
            │   └── daily_activity_2024.json
            ├── books/          # Only generated when both books+comics exist
            │   ├── week_0.json
            │   ├── ...
            │   └── daily_activity_2024.json
            └── comics/         # Only generated when both books+comics exist
                ├── week_0.json
                ├── ...
                └── daily_activity_2024.json
```
## Créditos

Este projeto é um fork do projeto original paviro/KoShelf.  
Agradecimentos especiais a:

Este projeto é um fork do original [paviro/KoShelf](https://github.com/paviro/KoShelf).

Agradecimentos especiais a:

[KoInsight](https://github.com/GeorgeSG/KoInsight) — pela inspiração de design.

[EventCalendar](https://github.com/vkurko/calendar) — pelo mecanismo de calendário.

[Tailwind CSS](https://tailwindcss.com/) — pelo framework de interface do usuário.

## Aviso Legal

Este é um projeto de fim de semana, desenvolvido para uso pessoal, e depende fortemente de código gerado por IA. Embora eu tenha testado tudo e o utilize diariamente, não me responsabilizo por quaisquer problemas que você possa encontrar. Use por sua conta e risco.
