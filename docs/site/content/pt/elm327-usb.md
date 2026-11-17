# ELM327 via USB

Um plugue para a tomada OBD com cabo até a porta USB. É o tipo de ELM327 mais confiável: sem pareamento, sem rádio, sem bateria descarregada.

## Driver

Dentro do plugue, um chip USB transforma o cabo numa porta serial (porta COM no Windows). O driver necessário depende desse chip:

| Chip | Driver |
|---|---|
| FTDI FT232 | normalmente instalado pelo próprio Windows, já incluído no Linux e no macOS |
| CH340 / CH341 | normalmente instalado pelo próprio Windows, já incluído no Linux e no macOS |
| Silicon Labs CP210x | do site do fabricante, se o Windows não encontrar |
| Prolific PL2303 | do site do fabricante; clones antigos muitas vezes só funcionam com um driver antigo |

Depois de conectar, o Gerenciador de Dispositivos do Windows mostra o adaptador em "Portas (COM e LPT)".

## Conectar

1. Conecte o adaptador no carro, ligue a ignição e conecte o cabo no computador.
2. Na página **Conectar**, escolha **Porta USB ou COM**.
3. Deixe a porta em **Procurar automaticamente** ou escolha na lista e conecte.

A busca automática testa todas as portas seriais nas velocidades comuns. Se você sabe a velocidade do seu adaptador, pode definir nas configurações em **Taxa de baud do adaptador**.

## Linux e macOS

No Linux, o adaptador aparece como `/dev/ttyUSB0` ou `/dev/ttyACM0`. Seu usuário precisa estar no grupo `dialout` (veja [Instalar](install.md)). No macOS, ele aparece como `/dev/cu.usbserial-...` ou `/dev/cu.wchusbserial...`.

## Problemas comuns

- **Nenhuma porta aparece**: falta o driver ou o cabo é só de carregamento.
- **O adaptador responde, o carro não**: ignição desligada, ou o adaptador é um clone que não dá conta do protocolo do carro. O [teste do adaptador](interfaces.md) conta mais.
- **Valores ao vivo muito lentos em carros mais antigos**: a própria K-Line é lenta, a culpa não é do adaptador.
