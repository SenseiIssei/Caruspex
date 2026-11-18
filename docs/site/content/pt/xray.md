# Vista de raio X

A **Vista de raio X** mostra o carro conectado como um modelo de vidro que você pode girar. Dentro você vê o motor ou os motores elétricos, o câmbio, o tanque ou a bateria de alta tensão, o escapamento, o radiador, a bateria de 12 V e a tomada OBD, dispostos de acordo com a carroceria e a tração do carro detectado.

![Vista de raio X](screenshot-xray.png)

## O que acende

- **Falhas** colorem a área delas, e a central responsável fica vermelha.
- **Valores ao vivo** aparecem como etiquetas na peça a que pertencem: líquido de arrefecimento no radiador, rotação no motor, tensão na bateria.
- **Centrais** encontradas pela busca ficam onde normalmente estão, ligadas pelas linhas de dados (CAN do powertrain, CAN de conforto, CAN de diagnóstico, K-Line). Enquanto o Caruspex lê, pequenos pacotes viajam até a central consultada.
- **Verificar carro** leva a câmera a cada achado, um depois do outro.

Gire com o mouse ou com o dedo, dê zoom com a rodinha ou com dois dedos; o teclado também funciona. As camadas (carroceria, trem de força, arrefecimento, combustível ou alta tensão, elétrica, rede) podem ser desligadas uma a uma.

## Quão exato é o modelo?

A forma vem das dimensões reais da geração, quando são conhecidas (do Wikidata ou dos dados técnicos do fabricante), e, caso contrário, de valores típicos para o tipo de carroceria e a época. Não é uma réplica do modelo exato: os dados de design e os diagramas elétricos dos fabricantes não estão disponíveis livremente. A página diz de onde vem a forma.

Se você tiver um modelo 3D do seu carro em glTF (`.glb` ou `.gltf`), pode carregá-lo com **Carregar o seu próprio modelo 3D**; o interior continua vindo da disposição.

## Sem 3D

Em computadores sem WebGL, a página passa para uma vista lateral plana com as mesmas informações.
