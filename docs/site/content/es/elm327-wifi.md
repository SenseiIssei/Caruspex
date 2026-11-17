# ELM327 por Wi-Fi

Algunos adaptadores abren su propia red Wi-Fi. El ordenador se une a ella y habla con el adaptador por TCP.

## Conectar

1. Enchufa el adaptador y pon el contacto. Al cabo de unos segundos aparece una red Wi-Fi, a menudo llamada `WiFi_OBDII`, `V-LINK` o como el fabricante.
2. Conecta el ordenador a esa red. No tiene internet, así que tu sistema puede avisar de que la conexión es limitada: no pasa nada.
3. En Caruspex elige **Wi-Fi** en la página **Conectar**. La mayoría de los adaptadores usan la dirección `192.168.0.10` y el puerto `35000`; viene ajustado de serie. Si el tuyo usa otra (por ejemplo `192.168.4.1`), escríbela como `address:port`.

## Consejos

- Mientras el ordenador está en la red del adaptador, no tiene internet. A Caruspex eso no le importa.
- Los adaptadores Wi-Fi funcionan en cualquier sistema sin controladores.
- Si la conexión se corta, acerca el ordenador al coche: la radio de estos conectores es débil.
