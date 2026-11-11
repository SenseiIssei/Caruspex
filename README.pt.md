# Caruspex

<!-- languages -->
[English](README.md) · [Deutsch](README.de.md) · [Español](README.es.md) · [Français](README.fr.md) · **Português** · [Italiano](README.it.md) · [Nederlands](README.nl.md) · [Polski](README.pl.md) · [Čeština](README.cs.md) · [Русский](README.ru.md) · [Українська](README.uk.md) · [Türkçe](README.tr.md) · [العربية](README.ar.md) · [中文](README.zh.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [हिन्दी](README.hi.md) · [Bahasa Indonesia](README.id.md)
<!-- /languages -->

[![Build](https://github.com/SenseiIssei/Caruspex/actions/workflows/build.yml/badge.svg)](https://github.com/SenseiIssei/Caruspex/actions/workflows/build.yml)
![Rust](https://img.shields.io/badge/Rust-2024-orange)
![Tauri 2](https://img.shields.io/badge/Tauri-2-24c8db)
![Platforms](https://img.shields.io/badge/platforms-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](LICENSE)

Diagnóstico de veículos de código aberto para Windows, Linux e macOS, escrito em Rust com uma interface Tauri 2.
Um cabo ou adaptador, um aplicativo: ler e apagar códigos de falha, acompanhar dados de sensores ao vivo,
verificar a prontidão para as emissões e conversar com as centrais da VW/Audi no nível do byte por um cabo K-Line.

O nome vem do *haruspex*, o adivinho romano que lia presságios nas entranhas. O Caruspex lê as do carro.

O aplicativo fala 18 idiomas, tanto a interface quanto cada texto dos códigos de falha: inglês, alemão, espanhol,
francês, português, italiano, holandês, polonês, tcheco, russo, ucraniano, turco, árabe (da direita para a esquerda), chinês,
japonês, coreano, hindi e indonésio. Ele segue o idioma do sistema, pode ser trocado nas configurações ou
forçado a cada execução com `--language fr` e assim por diante. Como as traduções são feitas e conferidas está em
[translations/TRANSLATING.md](translations/TRANSLATING.md).

![Visão geral com indicadores ao vivo](docs/screenshot-overview.png)

## Destaques

- **Duas pilhas de diagnóstico completas**: OBD-II/EOBD padrão com qualquer adaptador ELM327, e diagnóstico
  do fabricante da VW/Audi (KWP1281 e KWP2000) por um cabo KKL K-Line direto.
- **Protocolos implementados do zero**: o KWP1281 confirma cada byte com o seu complemento de um,
  algo que o firmware do ELM327 não consegue fazer. O Caruspex controla a K-Line por conta própria, incluindo o endereço de
  despertar a 5 baud, que é gerado bit a bit alternando o sinal de break da porta serial com temporização baseada em prazos, de modo que
  a oscilação do escalonador do sistema operacional não se acumula ao longo dos dez bits.
- **Detecção automática do protocolo**: depois do despertar, os dois bytes de chave da central dizem se ela fala
  KWP1281 ou KWP2000 (ISO 14230); o Caruspex muda de acordo.
- **Workspace Rust modular**: um crate por área (ligações com o adaptador, protocolos, rede e CAN, veículos, dados,
  gravações, análise, carro de demonstração, ferramenta de linha de comando) e o aplicativo Tauri por cima. O núcleo não sabe nada
  da interface, então o aplicativo e a ferramenta de linha de comando rodam exatamente o mesmo código.
- **Testado sem carro**: centrais simuladas (`crates/caruspex-protocols/tests/common/`) reproduzem o
  handshake byte a byte, gateways falsos fazem o papel de ENET e DoIP, um driver J2534 falso cobre o caminho PassThru
  e um simulador de ELM327 em Python permite execuções completas do aplicativo.
- **Um único conjunto de traduções**: o núcleo e a interface compartilham os mesmos arquivos de tradução em
  `translations/<code>/`, em 18 idiomas. O núcleo procura seus textos pelo texto em inglês, a interface monta
  seus catálogos a partir dos mesmos arquivos.
- **Reconhece qualquer carro**: um decodificador de VIN com 365 códigos de fabricante e um catálogo de mais de 2.100
  gerações de modelos desde 1980 dizem qual carro está conectado e o que funciona com ele.
- **Carro de demonstração integrado**: um ELM327 no próprio processo com um motor e uma caixa de câmbio em CAN, para experimentar tudo
  sem hardware.
- **Dados gerados, não digitados à mão**: cerca de 1.700 códigos de falha, muitos deles calculados a partir de famílias de códigos regulares
  (por cilindro, por banco, por sensor de oxigênio) com `tools/gen_dtc.py`.

## Funções

### Conhece o carro
- Depois de conectar, o Caruspex lê o VIN e diz qual é o carro: marca, modelo, geração, ano do modelo.
- Um catálogo de mais de 2.100 gerações de modelos de 94 marcas, de 1980 até hoje, incluindo marcas que não existem mais.
- Para cada geração: qual conector de diagnóstico, onde fica, qual protocolo e se OBD-II, um cabo KKL ou só as ferramentas do próprio fabricante funcionam.
- Um verificador de VIN para qualquer VIN: fabricante, país, ano do modelo, dígito verificador, código do modelo.
- Uma garagem que reconhece cada carro pelo VIN e guarda o histórico de suas verificações e leituras do odômetro.

![Veículos](docs/screenshot-vehicles.png)

### Verificação de saúde com um botão
- Lê a memória de falhas, os monitores de prontidão e os testes de componentes e depois acompanha a mistura, as temperaturas, os sensores de oxigênio e a tensão de carga por meio minuto.
- Uma nota de 0 a 100 em que cada ponto perdido é explicado em palavras simples, com o próximo passo.
- Pronto para o teste de emissões? Qual monitor ainda está aberto e como dirigir para concluí-lo.
- Um relatório de inspeção em PDF, para imprimir ou levar à oficina.

![Verificação de saúde](docs/screenshot-health.png)

### Memória de falhas
- Códigos armazenados, pendentes e permanentes; com várias centrais, cada código mostra quem o reportou (motor, caixa de câmbio).
- Cerca de 1.400 códigos padrão (P, C, B, U) mais códigos de fabricante para VAG, Ford, GM, Toyota, Honda, Nissan, Hyundai/Kia, Subaru e outros. Um código P1 é lido do jeito que a marca detectada o entende.
- Um semáforo por código e a pergunta que todo mundo faz primeiro: posso continuar dirigindo?
- Causas comuns, coisas que você mesmo pode verificar e uma faixa de custo aproximada.
- O carro em visão transparente mostra onde fica a peça afetada.
- Apagamento com um aviso claro e releitura automática; exportação em CSV.

![Memória de falhas](docs/screenshot-faults.png)

### O que devo fazer agora?
- Uma página que organiza tudo o que o Caruspex encontrou em três passos: parar já, logo para a oficina, pode esperar.
- Cada ponto com uma frase simples explicando o porquê, uma faixa de custo aproximada e uma nota para mostrar na oficina (código, central, área, causas comuns).
- O OBD não consegue ver se a luz do motor está piscando ou se há um aviso vermelho aceso: duas caixas de seleção perguntam isso, e uma luz piscando com falhas de ignição move o ponto para "parar já".

![O que devo fazer agora?](docs/screenshot-todo.png)

### Verificar um carro usado
- Para a visita antes da compra: o VIN do carro confere com os documentos e com o catálogo, a quilometragem coincide entre as centrais, as falhas foram apagadas pouco antes (distância e tempo desde o apagamento)?
- Monitores de prontidão, falhas armazenadas, a verificação de saúde e, em carros elétricos, o estado da bateria.
- Um semáforo, as perguntas para o vendedor e um PDF para a compra.

![Verificar um carro usado](docs/screenshot-used-car.png)

### Vista de raio X em 3D
- O carro conectado como um modelo de vidro que você pode girar: motor ou motores, câmbio, tanque ou bateria de alta tensão, escapamento, radiador, a bateria de 12 V e a tomada OBD, dispostos conforme a carroceria e a tração do carro detectado.
- Cada central encontrada na varredura fica onde normalmente está, ligada por suas linhas de dados (CAN do trem de força e de conforto); enquanto o Caruspex lê, pequenos pacotes viajam até a central consultada.
- As falhas iluminam sua área e deixam a central responsável vermelha, os valores ao vivo ficam na peça a que pertencem e "Verificar o carro" leva a câmera a cada achado. As camadas podem ser desligadas uma a uma.
- Dimensões típicas para a carroceria e a época, não uma réplica do modelo exato: os dados de projeto e os esquemas elétricos dos fabricantes não são de livre acesso.

![Vista de raio X](docs/screenshot-xray.png)

### Valores ao vivo
- Todos os valores do modo 01 da SAE J1979 até o PID C4: 145 PIDs com mais de 300 leituras, agrupadas e pesquisáveis.
- Odômetro, marcha, temperatura do óleo, consumo de combustível, torque, pressão do turbo, temperaturas de escape, filtro de partículas, AdBlue, NOx e bateria híbrida, conforme o carro informar.
- Gráfico contínuo e gravação em CSV.

![Valores ao vivo](docs/screenshot-live.png)

### Medir
- Computador de bordo: consumo ao vivo e médio, custo, CO2, distância e uma nota de condução suave, mesmo para carros sem computador de bordo próprio.
- Desempenho: 0 a 100 km/h, 80 a 120, quarto de milha e distância de frenagem a partir do sinal de velocidade (somente em vias fechadas).
- Bateria e alternador: tensão em repouso, queda na partida, tensão de carga com e sem carga, guiado em quatro passos.

### Inspeção
- Monitores de prontidão em palavras simples, quadro congelado, VIN.
- Monitoramento de componentes do modo 06 com valor e limites, contagem de falhas de ignição por cilindro.
- Nomes das centrais, IDs de calibração e somas de verificação do modo 09, por central.
- Terminal ELM327 bruto.

### Verificação do adaptador
- ELM327 original ou clone? Ele pede os comandos de cada versão, mede o tempo de resposta e diz com honestidade o que se conclui disso.
- Reconhece OBDLink e outros adaptadores STN.
- Nunca grava nada no adaptador.

![Verificação do adaptador](docs/screenshot-adapter.png)

### Todos os cabos, todos os caminhos para dentro do carro
O Caruspex descobre sozinho o que está conectado e o que o carro fala.

| Interface | Exemplos | Como |
|---|---|---|
| ELM327 por USB, Bluetooth, Bluetooth LE, Wi-Fi | ELM327 barato, Vgate, vLinker, Veepeak | porta serial, pareado ou BLE direto, TCP |
| Chips STN | OBDLink SX, EX, MX+, CX | como ELM327, com temporização mais rápida |
| SAE J2534 PassThru | Tactrix OpenPort 2.0, Mongoose, VCX Nano, Scanmatik, Godiag, VAS 5054A em modo J2534 | o driver de 64 bits do fabricante, CAN com ISO-TP feito pelo Caruspex |
| SLCAN / Lawicel | CANable, USBtin, CANUSB | porta serial, encontrada pela busca automática de portas |
| SocketCAN (Linux) | PEAK PCAN-USB, Kvaser, CANable com candleLight, placas CAN para Raspberry Pi | can0 e similares |
| Cabo KKL (VAG 409.1) | cabos FTDI ou CP2102 | K-Line, KWP1281 e KWP2000 |
| Cabo ENET | BMW e Mini a partir da série F | HSFZ sobre Ethernet |
| Cabo DoIP | muitos carros a partir de 2016 aproximadamente | ISO 13400 sobre Ethernet |

Em todas elas funcionam as mesmas funções: OBD-II e OBD sobre UDS, todas as centrais sobre UDS, endereçamento
BMW, escuta de valores de broadcast (Tesla), gravação e reprodução.

![Conectar](docs/screenshot-connect.png)

### Todas as centrais por CAN (UDS)
- Com um ELM327 comum, o Caruspex pergunta a cada central, uma por uma: motor, caixa de câmbio, ABS, airbag, painel de instrumentos, climatização e mais, na maioria dos carros a partir de 2008 aproximadamente.
- Memória de falhas por central com o tipo de defeito (curto-circuito, circuito aberto, implausível ...), número da peça, versão de software e de hardware.
- Listas de endereços para VAG, BMW e Mini, Ford, Mazda, GM, Toyota, Nissan, Hyundai/Kia, Renault e PSA; para todo o resto, uma varredura pelos endereços usuais.
- BMW e Mini a partir da série F com o endereçamento estendido próprio da BMW sobre 6F1, além de uma varredura por todos os 128 endereços de diagnóstico; códigos de falha no formato da BMW como o ISTA os mostra (p. ex. 480A10).
- Apagamento por central, com um aviso.

![Todas as centrais](docs/screenshot-ecus.png)

### VW e Audi por um cabo KKL (KWP1281 e KWP2000)
- Adaptação do corpo de borboleta: o ajuste básico de verdade no bloco de medição 060 ou 098.
- Varredura de todas as centrais, incluindo ABS, airbag, painel de instrumentos e conforto, com um relatório imprimível.
- Canais de adaptação e testes de saídas, de propósito somente leitura.

### Valores do fabricante e personalizados
- Baterias de carros elétricos: estado de saúde, carga, tensões das células com dispersão, temperaturas, para Hyundai Kona e Ioniq, Kia e-Niro, Renault Zoe, Nissan Leaf e Tesla Model 3 e Y (escutando o barramento CAN); cada valor com uma fonte documentada.
- Valores do fabricante para cerca de 550 veículos a mais, do projeto [OBDb](https://github.com/OBDb) (bateria, pressão dos pneus, óleo, odômetro e mais), oferecidos automaticamente para o carro detectado; cada equação é conferida com o decodificador do próprio OBDb.
- Carregue seus próprios valores no formato CSV do Torque, muito difundido, com um avaliador completo das equações do Torque.

![Bateria do EV](docs/screenshot-battery.png)

### Gravar e reproduzir
- Grave uma sessão com o carro em um arquivo e reproduza-a depois sem o carro, útil para localizar problemas.

### Sem carro nem adaptador
- Um carro de demonstração integrado: ele anda, esquenta, tem algumas falhas armazenadas e fala como um ELM327 de verdade por CAN.
- Um léxico offline de todos os códigos, filtrável por marca.

### Aparência e uso
- Uma interface nova no estilo do Odysync: calma, escura, com transições suaves.
- Conexão passo a passo com uma busca automática em todas as portas e, quando falha, uma lista de verificação curta para esse tipo de conexão (driver, pareamento, ignição, taxa de bits, adaptadores clones) com o que o Caruspex já descobriu.
- Modo iniciante e profissional: quem está começando vê as páginas do dia a dia; as ferramentas de oficina (KKL, adaptação, comandos brutos, endereços) aparecem no modo profissional.
- 18 idiomas; as configurações valem imediatamente.
- Avisa quando há uma versão nova: ao iniciar, pergunta ao GitHub pela versão mais recente e nada mais (nenhum dado
  sobre você ou o carro); pode ser desligado nas configurações.

## Veículos compatíveis

Qualquer veículo com OBD-II ou EOBD: na Europa, carros a gasolina a partir de 2001 e diesel a partir de 2004; nos EUA, a partir de 1996. VW, Audi, Seat e Skoda mais antigos, a partir de 1990 aproximadamente, funcionam pelo cabo KKL. A página Veículos diz qual caminho funciona para um determinado carro.

| Época | Barramento | Exemplos |
|---|---|---|
| até 1995 aproximadamente | específico do fabricante | VAG com KKL pelos conectores 2x2, nos demais só ferramentas de oficina |
| de 1996 a 2003 aproximadamente | K-Line (ISO 9141-2, KWP2000) | Audi A2, VW Golf IV, Toyota, Renault |
| a partir de 2004 aproximadamente | CAN (ISO 15765-4) | Audi A4 B7, VW Golf V, carros japoneses modernos |
| EUA até 2005 aproximadamente | SAE J1850 PWM e VPW | Ford, Chevrolet |
| carros mais novos | OBD sobre UDS (SAE J1979-2) | obrigatório nos EUA a partir do ano-modelo 2027, já presente em alguns carros recentes; detectado e traduzido automaticamente |
| BMW e Mini a partir da série F | ENET (HSFZ sobre Ethernet) ou D-CAN com endereçamento 6F1 | todas as centrais |
| muitos carros a partir de 2016 aproximadamente | DoIP (ISO 13400) com um cabo DoIP | OBD sobre Ethernet, encontrado automaticamente |
| Tesla Model 3 e Y | sem OBD: quadros CAN de broadcast | bateria, células, temperaturas, velocidade, odômetro por escuta (cabo adaptador para o conector de diagnóstico) |

Carros totalmente elétricos não precisam oferecer dados de emissões OBD-II. Alguns respondem em parte mesmo assim; para carros que não respondem mas têm tráfego no barramento, o Caruspex conecta para escutar e lê o que os pacotes integrados conhecem (por enquanto Tesla Model 3 e Y).

## Adaptadores compatíveis

- Qualquer adaptador compatível com **ELM327**: USB, Bluetooth clássico (diretamente por RFCOMM no Windows e no Linux depois de pareado, ou pela sua porta COM), **Bluetooth LE**
  (Vgate iCar Pro BLE, vLinker, OBDLink CX, Veepeak BLE e outros adaptadores com os serviços GATT usuais; encontrados por uma
  busca, sem pareamento) ou Wi-Fi por TCP
- Um **cabo ENET** (OBD para RJ45) para BMW e Mini a partir da série F: o Caruspex encontra o carro na rede
  (HSFZ, portas 6811 e 6801) e lê todas as centrais por Ethernet, sem chip adaptador no meio. O computador
  precisa de um endereço automático (169.254.x.x), que Windows, macOS e Linux atribuem sozinhos quando nenhum DHCP responde.
  Aqui também o Caruspex apenas lê; codificação e gravação de firmware ficam com o E-Sys, o ISTA ou as ferramentas de tuning.
- Qualquer **cabo KKL (VAG 409.1)**. Cabos baseados em FTDI ou CP2102 são muito mais confiáveis na K-Line do que os
  clones mais baratos; para KWP1281 recomenda-se um FTDI FT232RL original.

Interfaces proprietárias como Delphi DS150E, Autocom ou VAS 5054A não são compatíveis, elas não falam ELM327.

| Função | ELM327 | KKL 409.1 |
|---|---|---|
| Ler e apagar falhas do motor | sim | sim |
| Valores ao vivo, quadro congelado, prontidão | sim | sim |
| ABS, airbag, painel de instrumentos, conforto | **não** | sim |
| Blocos de medição do fabricante | **não** | sim |
| **Adaptação do corpo de borboleta** | **não** | sim |

Isso é um limite do hardware, não do software: o KWP1281 é um protocolo de blocos em que cada byte é
confirmado pelo outro lado. O firmware do ELM327 só trata pares completos de requisição e resposta.
Um cabo KKL repassa a K-Line crua, então o Caruspex pode falar o protocolo por conta própria.

### Dois protocolos VAG, detectados automaticamente

| Protocolo | Típico de | Ajustes básicos |
|---|---|---|
| KWP1281 (protocolo de blocos) | motores 1.4 MPI | comando próprio (bloco 0x28) |
| KWP2000 (ISO 14230) | 1.6 FSI com Bosch MED | sessão de diagnóstico própria e depois leitura de bloco |

Os canais de adaptação e os testes de saídas estão implementados por enquanto no caminho KWP1281; no KWP2000 o
aplicativo avisa em vez de falhar em silêncio.

Ainda não implementado: diagnóstico do fabricante por CAN com VW TP 2.0 (veículos a partir de 2004 aproximadamente).

## Tecnologia

| Área | Tecnologia |
|---|---|
| Linguagem | Rust (edição 2024) |
| Interface | Tauri 2 com React 19 e TypeScript, compilada com Vite |
| Bibliotecas | `serialport` (portas seriais), `btleplug` (Bluetooth LE), `libloading` (drivers J2534) |
| Localização | Um único conjunto de arquivos JSON em `translations/` para núcleo e interface, 18 idiomas |
| Compilação | Workspace do Cargo, npm para a interface |
| Testes | `cargo test` contra centrais simuladas, gateways falsos e um driver J2534 falso |
| Ferramentas | Python 3 (geradores de código e tabelas, verificações de tradução, simulador de ELM327) |
| Plataformas | Windows 10 e 11, Linux, macOS |

## Download

Pacotes prontos estão na [página de versões](https://github.com/SenseiIssei/Caruspex/releases):

| Sistema | Arquivo | Observações |
|---|---|---|
| Windows 10 e 11, 64 bits | `Caruspex-<version>-windows-x64-setup.exe` | Instala sem direitos de administrador e pode colocar `caruspex_cli` no caminho de busca. Atualiza uma 0.8 já instalada no lugar e instala o ambiente WebView2 só se ele estiver faltando |
| Windows 10 e 11, 64 bits | `Caruspex-<version>-windows-x64.zip` | Portátil, descompacte e inicie. Precisa do ambiente WebView2, que o Windows 10 e 11 já têm |
| macOS 13 e mais novo, Apple Silicon | `Caruspex-<version>-macos.dmg` | Arraste o aplicativo para Aplicativos |
| Linux, x86_64 | `Caruspex-<version>-linux-x86_64.AppImage` | `chmod +x` e inicie |
| Debian e Ubuntu, 64 bits | `Caruspex-<version>-linux-amd64.deb` | Instale com `sudo apt install ./Caruspex-<version>-linux-amd64.deb` |
| Somente a ferramenta de linha de comando | `caruspex_cli-<version>-windows-x64.zip`, `caruspex_cli-<version>-linux-x86_64.tar.gz`, `caruspex_cli-<version>-macos.tar.gz` | Para scripts e servidores, sem interface |

Os pacotes ainda não são assinados. O Windows SmartScreen pergunta na primeira inicialização ("Mais informações", "Executar mesmo assim"); no macOS,
abra o aplicativo na primeira vez com o botão direito e "Abrir".

## Compilação

Requisitos: a toolchain estável do Rust (edição 2024, Rust 1.85 ou mais novo), Node 24 com npm e Python 3 para
os geradores e as verificações de tradução. No Linux, os pacotes do sistema de que a interface e as ligações com o adaptador precisam
(nomes do Debian e do Ubuntu):

```bash
sudo apt-get install libwebkit2gtk-4.1-dev librsvg2-dev libdbus-1-dev libudev-dev pkg-config
```

O Windows precisa do ambiente WebView2 (parte do Windows 10 e 11); o macOS não precisa de nada a mais.

Instale uma vez as dependências da interface e depois compile a ferramenta de linha de comando e o aplicativo:

```bash
npm --prefix app ci
```

```bash
cargo build --release -p caruspex-cli
```

```bash
npm --prefix app run tauri -- build
```

`npm --prefix app run tauri -- dev` inicia o aplicativo com recarga ao vivo enquanto você trabalha nele. A
interface sozinha, sem a janela do Tauri, roda em um navegador comum com o carro de demonstração integrado:

```bash
npm --prefix app run dev
```

Abra `http://localhost:5287/?demo` para conectar ao carro de demonstração na hora. `?lang=de` escolhe um idioma e
`?theme=light` o tema claro, por exemplo `?demo&lang=de&theme=light`. Sem o shell do Tauri não há
acesso ao adaptador, então a versão de navegador só conhece o carro de demonstração.

### Permissões da porta serial no Linux

Para acessar o adaptador sem `sudo`:

```bash
sudo usermod -aG dialout $USER
```

Depois saia da sessão e entre de novo.

## Saídas da compilação

| Saída | Descrição |
|---|---|
| `target/release/caruspex` / `caruspex.exe` | O aplicativo com a interface Tauri (`tauri build` também grava os instaladores e pacotes em `target/release/bundle/`) |
| `target/release/caruspex_cli` / `caruspex_cli.exe` | Ferramenta de linha de comando automatizável sobre o mesmo núcleo, veja [Ferramenta de linha de comando](#ferramenta-de-linha-de-comando) |

### Pacotes e versões

`bash tools/package.sh` monta em `dist/` os pacotes do sistema atual: o instalador e dois zips no
Windows (o instalador precisa do [Inno Setup 6](https://jrsoftware.org/isinfo.php)), o dmg e o arquivo da linha de comando
no macOS, o AppImage, o `.deb` e o arquivo da linha de comando no Linux. Depois `bash tools/smoke_test.sh` os instala ou
abre como um usuário faria, inicia os dois programas e os remove de novo. A CI faz as duas coisas a cada execução.

Uma versão sai de uma tag. `bash tools/release.sh <version>` a prepara em uma branch própria sem enviar nada:
ele define a versão em `Cargo.toml` e `app/package.json`, cria `changelog/<version>.md` (alemão e inglês),
executa as verificações, os testes, os pacotes e o teste de fumaça, e atualiza as capturas de tela do README (`tools/screenshots.sh de` e
`tools/screenshots.sh en`). Depois que o pull request é mesclado, crie a tag `v<version>` e envie a tag. A CI compila os
três sistemas e publica a versão com os pacotes e a lista de mudanças.

A CI (`.github/workflows/build.yml`) executa primeiro a formatação, a verificação das tabelas geradas, a verificação das traduções, a
compilação da interface e o clippy no Linux, e depois testa, empacota e faz o teste de fumaça no Windows, Linux e macOS.
Um segundo workflow (`.github/workflows/data-check.yml`) procura uma vez por mês novos códigos VIN, veículos e arquivos DBC
na origem e abre uma issue com as novidades.

## Testes

```bash
cargo test --workspace
```

Cerca de 370 testes rodam em poucos segundos, todos sem carro. Eles cobrem o analisador de respostas do ELM327, a decodificação
de códigos de falha, a conversão de PIDs, os perfis de veículos e o decodificador de VIN, os serviços OBD-II e UDS contra o carro de demonstração,
sessões completas de KWP1281 e KWP2000 contra centrais simuladas, ENET (HSFZ) e DoIP contra gateways falsos,
SLCAN e o caminho J2534 contra um driver falso, gravações e reprodução, e a análise (verificação de saúde, teste de bateria,
computador de bordo, cronômetro de desempenho). Os testes da linha de comando executam o `caruspex_cli` compilado e comparam a saída com
os arquivos de referência em `crates/caruspex-cli/tests/golden/`, que guardam o que a versão antiga em C++ imprimia para os mesmos
comandos, de modo que qualquer diferença em relação à ferramenta antiga aparece.

As verificações que a CI executa além dos testes:

```bash
cargo fmt --all --check
```

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

```bash
python tools/i18n_rust.py --strict
```

```bash
npm --prefix app run build
```

As duas primeiras são a formatação e os avisos do linter. `tools/i18n_rust.py --strict` falha quando um texto do núcleo está faltando nos
arquivos de tradução ou uma tradução está faltando em um dos 18 idiomas; a compilação da interface confere os próprios
textos do mesmo jeito quando `I18N_STRICT=1` está definido. Execute `python tools/gen_dtc.py`, `gen_vehicles.py`, `gen_ecus.py` e
`gen_pids.py` para atualizar as tabelas geradas; a CI falha se elas diferirem do que está versionado.

## Experimente sem carro

O jeito mais rápido é o carro de demonstração integrado: escolha "Carro de demonstração" na página Conectar, ou inicie com

```bash
target/release/caruspex --demo --language en
```

`caruspex --help` lista todas as opções do aplicativo: `--tcp`, `--port`, `--bluetooth`, `--enet`, `--can`,
`--replay`, `--report`, `--language`, `--theme`, `--page`, `--shot` e `--size`.

O simulador em Python faz o papel do motor do Audi A2 na K-Line ou, com `--can`, de um carro CAN com motor
e caixa de câmbio (cabeçalhos, ISO-TP, modos 06 e 09 por central):

```bash
python tools/mock_elm327.py --can
```

Depois conecte o aplicativo (no Windows o binário é `target/release/caruspex.exe`):

```bash
target/release/caruspex --tcp 127.0.0.1:35000 --language en
```

A ferramenta de linha de comando também funciona contra o simulador:

```bash
target/release/caruspex_cli read --tcp 127.0.0.1:35000
```

### Ferramenta de linha de comando

`caruspex_cli <command>` com `--demo`, `--tcp HOST[:PORT]`, `--port NAME [--baud N]`, `--enet HOST|auto`,
`--can SPEC` ou `--replay FILE`:

| Comando | O que faz |
|---|---|
| `read` | Luz de aviso, prontidão e códigos de falha, com o significado para a marca quando o VIN a revela |
| `vin` | VIN, o que ele diz (fabricante, país, ano do modelo, dígito verificador) e o carro correspondente do catálogo |
| `freeze` | Todos os quadros congelados, um por falha armazenada |
| `health` | Verificação de saúde: falhas, prontidão, modo 06 e uma série ao vivo (`--seconds N`, 15 por padrão) |
| `probe` | Teste do adaptador: o que o ELM327 realmente é, quais comandos conhece, tempos de ida e volta |
| `uds` | UDS por CAN: identificação e memória de falhas de cada central (`--ids A,B,...` para seus próprios endereços) |
| `custom` | Valores do fabricante e personalizados de um pacote integrado (`--pack NAME`) ou de um CSV do Torque (`--csv FILE`), uma vez |
| `live`, `info`, `clear`, `ecus`, `ports`, `kklcheck` | Valores ao vivo, VIN e primeiro quadro congelado, apagamento, tudo por central, portas seriais, autoteste do cabo KKL |
| `enet`, `interfaces` | Procura carros BMW e Mini e carros DoIP na rede; lista as interfaces CAN deste computador (drivers J2534, SocketCAN) |
| `version`, `help` | Versão do programa, a lista completa de comandos e opções |

`--json` imprime `read`, `vin`, `health`, `probe`, `freeze`, `uds` e `custom` como JSON para scripts, `--record FILE`
salva a sessão inteira com o adaptador para `--replay`, `--fast-timing` coloca o adaptador em
temporização adaptativa agressiva, `-v` registra cada comando do adaptador (no stderr junto com `--json`). `caruspex_cli help` lista tudo.

```bash
target/release/caruspex_cli health --demo --json > health.json
```

As capturas de tela desta página vêm do carro de demonstração, tiradas pelo próprio aplicativo:
`--demo --page faults --shot shot.png --size 1440x920` abre uma página, salva uma imagem da janela depois de alguns
segundos e sai (`tools/screenshots.sh` tira o conjunto completo).

## Estrutura do projeto

```
crates/caruspex-i18n/       Idiomas: alemão e inglês no código, os outros 16 vindos de translations/
crates/caruspex-vehicles/   Decodificador de VIN e catálogo de veículos (marcas, modelos, gerações, conectores, barramentos)
crates/caruspex-data/       Códigos de falha, PIDs, endereços de centrais, mapeamento de código para componente
crates/caruspex-link/       Caminhos até o adaptador (serial, TCP, Bluetooth LE), driver ELM327, verificação do adaptador
crates/caruspex-protocols/  OBD-II, UDS, KWP2000 e KWP1281 (K-Line, byte a byte)
crates/caruspex-net/        BMW ENET (HSFZ), DoIP, SLCAN, SocketCAN, J2534 PassThru
crates/caruspex-record/     Gravações e reprodução, valores personalizados (CSV do Torque, pacotes de PIDs), gravador de JSON
crates/caruspex-analysis/   Verificação de saúde, guia do ciclo de condução, computador de bordo, cronômetro de desempenho, teste de bateria
crates/caruspex-demo/       O carro de demonstração integrado: um ELM327 com centrais simuladas
crates/caruspex-cli/        A ferramenta de linha de comando, com seus testes de arquivos de referência
app/src/                    A interface: React 19 e TypeScript (páginas, estrutura, estado, relatório, i18n)
app/src-tauri/              O aplicativo Tauri: janela, comandos, verificação de atualização, opções de linha de comando
translations/               Arquivos de tradução compartilhados para núcleo e interface, 18 idiomas
data/                       O catálogo de veículos como CSV mantido à mão, fontes de centrais e PIDs
tools/                      Geradores, verificações de tradução, empacotamento, scripts de versão e capturas, simulador de ELM327
docs/                       Capturas de tela do README
packaging/                  Script do instalador do Windows, arquivo desktop do Linux, Info.plist do macOS
changelog/                  Notas de versão por versão, alemão e inglês
```

O banco de dados de códigos de falha é gerado: `tools/gen_dtc.py` expande as famílias regulares de códigos EOBD e as
listas selecionadas em `tools/dtc/` para `crates/caruspex-data/src/generated/*.rs`. Execute-o de novo depois de mudar os dados:

```bash
python tools/gen_dtc.py
```

O catálogo de veículos funciona do mesmo jeito: edite os arquivos CSV em `data/vehicles/` e depois execute

```bash
python tools/gen_vehicles.py
```

Endereços de centrais e pacotes de PIDs têm `tools/gen_ecus.py` e `tools/gen_pids.py`. As traduções estão descritas
em [translations/TRANSLATING.md](translations/TRANSLATING.md); depois de adicionar um texto no código,
`python tools/i18n_rust.py` lista o que ainda falta traduzir.

## Uma observação sobre apagar códigos

O modo 04 do OBD apaga a memória de falhas, o quadro congelado e os monitores de prontidão. Logo depois, o
carro será reprovado em um teste de emissões até concluir vários ciclos de condução completos, e um código cuja causa
ainda existe voltará depois de um trajeto curto. Apagar não é consertar.

## Aviso legal

O Caruspex é um projeto de hobby e é fornecido sem garantia. Use por sua conta e risco, somente em veículos
em que você tem permissão para trabalhar e nunca enquanto dirige. Todos os nomes de produtos e marcas são marcas registradas de seus
respectivos proprietários; o Caruspex não é afiliado nem endossado por nenhum fabricante de veículos.

## Licença

MIT, veja [LICENSE](LICENSE). Os valores do fabricante de centenas de modelos são convertidos do [OBDb](https://github.com/OBDb) e, como o OBDb, licenciados sob CC BY-SA 4.0: ficam separados do código em [data/obdb](data/obdb) com a licença e a lista de fontes ([ATTRIBUTION.md](data/obdb/ATTRIBUTION.md)). As fontes incluídas são licenciadas sob a SIL Open Font License 1.1:
JetBrains Mono ([OFL.txt](resources/fonts/OFL.txt)), Inter ([OFL-Inter.txt](resources/fonts/OFL-Inter.txt)) e
Plus Jakarta Sans ([OFL-PlusJakartaSans.txt](resources/fonts/OFL-PlusJakartaSans.txt)).
