# Segurança e atualizações

Esta página reúne as informações de segurança do Caruspex: para que serve o programa, que riscos convém conhecer,
como funcionam as atualizações e como removê-lo. Segue as informações para utilizadores exigidas pelo regulamento
europeu de ciber-resiliência (Cyber Resilience Act, anexo II), na medida em que se aplicam a um programa livre e não
comercial.

## O programa

- **Nome**: Caruspex, a aplicação `caruspex` e a ferramenta de linha de comandos `caruspex_cli`, para Windows, Linux
  e macOS. A versão está no nome de cada download e na lista de alterações de cada versão.
- **Publicado por**: o projeto Caruspex no [GitHub](https://github.com/SenseiIssei/Caruspex), como software livre sob
  a licença MIT. Perguntas e erros vão para as [issues](https://github.com/SenseiIssei/Caruspex/issues).
- **Finalidade**: ler e explicar os dados de diagnóstico de carros, motos e camiões com o seu próprio adaptador, no
  seu próprio computador. Sem conta, sem nuvem e sem rastreamento ([Privacidade](privacy.md)).
- **Propriedades de segurança**: ler nunca altera nada no carro, e tudo o que faz o carro agir pergunta antes. Uma
  atualização só é instalada se a assinatura corresponder à chave integrada na aplicação. O Caruspex não abre portas
  de rede nem executa código de ficheiros ou da internet.

## Comunicar uma vulnerabilidade {#report}

Comunique problemas de segurança em privado com o
[formulário de novo aviso de segurança](https://github.com/SenseiIssei/Caruspex/security/advisories/new), não numa
issue pública. A [política de segurança](https://github.com/SenseiIssei/Caruspex/blob/main/SECURITY.md) explica o que
acontece depois. As vulnerabilidades corrigidas são publicadas como
[avisos de segurança](https://github.com/SenseiIssei/Caruspex/security/advisories).

## Riscos a conhecer {#risks}

- Descarregue o Caruspex apenas da [página de versões](https://github.com/SenseiIssei/Caruspex/releases). Cópias de
  outros sítios podem ter sido alteradas.
- Abra gravações, pacotes de PID e modelos 3D só se vierem de pessoas de confiança. Não podem executar código, mas um
  ficheiro danificado pode tornar o Caruspex lento ou fechá-lo.
- Apague códigos de avaria, inicie testes de atuadores e ajustes básicos só com o carro parado e num local seguro.
- Muitos adaptadores Bluetooth e Wi-Fi não têm PIN ou têm uma rede aberta: qualquer pessoa por perto se pode ligar.
  Desligue o adaptador quando terminar.
- Cabos ENET, DoIP e adaptadores Wi-Fi não cifram nada: use-os diretamente ou apenas na sua própria rede.
- Os controladores J2534 são programas dos seus fabricantes e correm com os seus direitos. Instale-os só a partir do
  fabricante.

## Atualizações e suporte {#support}

- Em cada instalação, também depois de atualizar a partir de uma versão mais antiga, o Caruspex pergunta uma vez "Procurar atualizações?". Só depois de um sim pergunta ao GitHub se há uma versão mais recente, ao arrancar e a cada seis horas enquanto está em execução. Uma versão nova apenas acende um botão na barra de título e nunca interrompe. Instalar é um clique, e a aplicação só instala um pacote cuja assinatura corresponda. O AppImage e a aplicação para macOS atualizam-se sozinhos; o pacote .deb e a aplicação Android recebem a página de download.
- A procura e a instalação automática são interruptores nas definições, em **Janela e idioma**, e podem ser alterados a qualquer momento. Sem um sim, nenhum pedido vai para o GitHub; então consulte de vez em quando a página de versões.
- As correções de segurança só entram na versão mais recente. Versões antigas não recebem correções à parte, por
  isso mantenha o Caruspex atualizado.
- O Caruspex é mantido por uma só pessoa, nos tempos livres, tanto quanto o tempo permitir e sem qualquer promessa: sem período de suporte nem prazos de resposta. Se a manutenção parar, o repositório será arquivado e dirá isso.

## Verificar um download {#verify}

Cada versão tem um ficheiro `SHA256SUMS` com a soma de verificação de cada ficheiro e o registo assinado do GitHub
de como os ficheiros foram construídos. Com os ficheiros numa mesma pasta:

```
sha256sum --ignore-missing -c SHA256SUMS
gh attestation verify Caruspex-<version>-windows-x64-setup.exe --repo SenseiIssei/Caruspex
```

Sem estas ferramentas, `Get-FileHash <ficheiro>` no PowerShell (Windows) ou `shasum -a 256 <ficheiro>` (macOS)
mostra a soma para comparar com a linha de `SHA256SUMS`.

## Remover o Caruspex e os seus dados {#remove}

1. Desinstale o programa: no Windows em Definições, Aplicações; no macOS arraste o Caruspex para fora de
   Aplicações; no Linux apague o AppImage ou use `sudo apt remove caruspex`.
2. Apague os seus dados: o botão **Abrir local de armazenamento** nas definições mostra as pastas com as definições,
   a garagem e os relatórios de falhas. Apague-as. Gravações e relatórios que guardou estão onde os guardou.

## Para programadores e empresas {#integrators}

- Cada versão tem, por plataforma, uma lista de materiais de software `Caruspex-<version>-sbom-<platform>.cdx.json`
  (CycloneDX) com cada biblioteca e a sua versão.
- A avaliação de riscos, a lista de requisitos e o âmbito do Cyber Resilience Act para o Caruspex estão em
  [docs/cra](https://github.com/SenseiIssei/Caruspex/tree/main/docs/cra).
- O Caruspex não é colocado no mercado como produto no sentido do Cyber Resilience Act, por isso não há declaração UE
  de conformidade nem marcação CE.
