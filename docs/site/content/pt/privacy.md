# Privacidade

O Caruspex trabalha no seu computador. Ele não tem conta, nem nuvem, nem rastreamento.

## O que é guardado

Tudo fica no seu computador:

| O quê | Quando | Onde |
|---|---|---|
| Configurações | sempre | a pasta de configurações do seu sistema (o botão **Abrir local de armazenamento** nas configurações mostra qual é) |
| Garagem: VIN, carro, verificações, leituras do hodômetro, a última conexão que funcionou | depois de conectar um carro | ao lado das configurações, na pasta de dados do Caruspex |
| Gravações e arquivos CSV | só quando você inicia | onde você salvar |
| Arquivo de log | só se você ativar nas configurações | o caminho que você escolher |
| Relatórios de travamento | só se o Caruspex travar | a pasta `crash-reports` ao lado da garagem |

Você pode apagar qualquer um desses arquivos a qualquer momento.

## O que é enviado

Ao iniciar, o Caruspex pergunta ao GitHub se existe uma versão mais nova. Nada sobre você ou o carro vai junto. A verificação pode ser desligada nas configurações. Fora isso, o Caruspex não se conecta à internet. A conversa com um adaptador Wi-Fi, um cabo ENET ou um carro DoIP fica na sua rede local.

## Relatórios de travamento só a pedido

Se o Caruspex travar, ele escreve um relatório curto no seu computador: versão, sistema, o ponto no código e a cadeia de chamadas. VINs, os bytes da comunicação com o carro e a sua pasta pessoal são cortados. Nada é enviado. O Caruspex mostra o relatório e oferece dois botões: **Relatar no GitHub** abre no navegador uma issue já preenchida, que você lê e altera antes de publicar, **Não relatar** deixa o relatório de lado.

## Compartilhar gravações sem dados pessoais

Gravações de carros reais ajudam a deixar o Caruspex melhor para mais carros. **Compartilhar gravação** prepara uma cópia e mostra exatamente o que fica e o que sai:

- **Removido**: os últimos dígitos do VIN (em todo o arquivo), o nome da porta e o endereço Bluetooth ou de rede do adaptador, o horário.
- **Nunca numa gravação**: sua localização e seus dados de usuário.
- **Fica**: os pedidos e as respostas, o tipo de adaptador, o protocolo e a data.

O arquivo fica no seu computador até você mesmo salvar e anexar a uma issue no GitHub. Nada é enviado automaticamente.

## O programa de testes

O [formulário de relatório de teste](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml) pergunta o carro, o ano, o motor e o adaptador. Ele só pede um VIN encurtado; nunca publique o VIN completo, sua placa ou seu nome.
