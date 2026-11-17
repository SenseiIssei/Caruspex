# ELM327 via Wi-Fi

Alguns adaptadores abrem uma rede Wi-Fi própria. O computador entra nessa rede e conversa com o adaptador via TCP.

## Conectar

1. Conecte o adaptador e ligue a ignição. Depois de alguns segundos aparece uma rede Wi-Fi, muitas vezes chamada `WiFi_OBDII`, `V-LINK` ou com o nome do fabricante.
2. Entre nessa rede com o computador. Ela não tem internet, então o sistema pode avisar que a conexão é limitada: tudo bem.
3. No Caruspex, escolha **Wi-Fi** na página **Conectar**. A maioria dos adaptadores usa o endereço `192.168.0.10` e a porta `35000`; isso já vem preenchido. Se o seu usar outro (por exemplo `192.168.4.1`), digite como `address:port`.

## Dicas

- Enquanto o computador estiver na rede do adaptador, ele fica sem internet. Para o Caruspex, isso não faz diferença.
- Adaptadores Wi-Fi funcionam em qualquer sistema, sem drivers.
- Se a conexão cair, aproxime o computador do carro: o rádio desses plugues é fraco.
