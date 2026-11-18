# ELM327 přes Wi-Fi

Některé adaptéry vytvoří vlastní síť Wi-Fi. Počítač se k ní připojí a s adaptérem komunikuje přes TCP.

## Připojení

1. Zapojte adaptér a zapněte zapalování. Po pár sekundách se objeví síť Wi-Fi, často s názvem `WiFi_OBDII`, `V-LINK` nebo podle výrobce.
2. Připojte počítač k této síti. Nemá internet, takže systém může upozornit, že připojení je omezené: to je v pořádku.
3. V Caruspexu na stránce **Připojit** vyberte **Wi-Fi**. Většina adaptérů používá adresu `192.168.0.10` a port `35000`; to je přednastavené. Pokud ten váš používá jinou (například `192.168.4.1`), zadejte ji jako `address:port`.

## Tipy

- Dokud je počítač v síti adaptéru, nemá internet. Caruspexu to nevadí.
- Adaptéry Wi-Fi fungují v každém systému bez ovladačů.
- Pokud se spojení přerušuje, přesuňte počítač blíž k autu: rádio těchto adaptérů je slabé.
