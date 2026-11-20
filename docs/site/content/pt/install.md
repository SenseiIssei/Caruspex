# Instalar

Todas as versões estão na [página de versões no GitHub](https://github.com/SenseiIssei/Caruspex/releases/latest). Os pacotes ainda não são assinados, por isso o Windows e o macOS perguntam uma vez antes da primeira execução.

## Windows 10 e 11

| Arquivo | O que é |
|---|---|
| `Caruspex-<version>-windows-x64-setup.exe` | O instalador. Não precisa de direitos de administrador, pode colocar a ferramenta de linha de comando `caruspex_cli` no caminho de busca e só instala o runtime WebView2 se ele estiver faltando. |
| `Caruspex-<version>-windows-x64.zip` | Portátil: descompacte e inicie o `caruspex.exe`. |

Na primeira execução, o Windows SmartScreen pode dizer que o aplicativo é desconhecido: clique em "Mais informações" e depois em "Executar assim mesmo".

Adaptadores USB precisam de um driver para o chip USB. O Windows 10 e 11 normalmente encontram sozinhos; se não, veja [ELM327 via USB](elm327-usb.md).

## macOS 13 e mais novo (Apple Silicon)

Baixe o `Caruspex-<version>-macos.dmg`, abra e arraste o Caruspex para Aplicativos. Como o aplicativo ainda não é notarizado, abra na primeira vez com o botão direito e "Abrir".

## Linux (x86_64)

| Arquivo | O que é |
|---|---|
| `Caruspex-<version>-linux-x86_64.AppImage` | Roda em quase todas as distribuições: torne executável com `chmod +x` e inicie. Ele também se atualiza sozinho. |
| `Caruspex-<version>-linux-amd64.deb` | Para Debian e Ubuntu: `sudo apt install ./Caruspex-<version>-linux-amd64.deb` |

Para usar adaptadores seriais sem `sudo`, adicione seu usuário ao grupo `dialout` uma vez, depois saia da sessão e entre de novo:

```
sudo usermod -aG dialout $USER
```

## Só a ferramenta de linha de comando

Para scripts e servidores existe o `caruspex_cli` sem a interface, para Windows, Linux e macOS. `caruspex_cli help` lista os comandos.

`caruspex_cli report` grava o relatório de diagnóstico num arquivo PDF, sem o app e por qualquer conexão que os outros comandos conhecem (`--demo`, `--port`, `--tcp`, `--enet`, `--can`, `--replay`):

```
caruspex_cli report --demo report.pdf
caruspex_cli report --port COM5 --lang de report.pdf
```

O relatório pode sair em alemão, inglês, espanhol, francês, português, italiano, neerlandês e indonésio (`--lang`). Nos outros idiomas ele sai em inglês, porque às fontes que todo leitor de PDF traz faltam as letras deles.

## Atualizações

Em cada instalação o Caruspex pergunta uma vez se deve procurar atualizações. Só depois de um sim pergunta ao GitHub se existe uma versão mais nova, e mais nada. Uma versão nova só é instalada se a assinatura dela bater com a chave embutida no aplicativo. A verificação e a instalação automática podem ser alteradas nas configurações a qualquer momento.

## Gerenciadores de pacotes

Pacotes para winget, Homebrew, Flathub e Snap estão sendo preparados. Até serem publicados, os arquivos acima são o jeito de instalar o Caruspex.
