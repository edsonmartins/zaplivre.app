# Testes Manuais (por plataforma)

Checklist detalhado para validacao manual antes de release.

## Cenarios de 2 devices (saidos da automacao Maestro)

Estes fluxos foram **removidos/adaptados na suite Maestro** porque exigem um
peer real alcancavel (nao sao cobriveis com 1 device). Ver
`e2e/maestro/README.md` -> "Limitacoes conhecidas com 1 device".

Preparo: dois devices (A e B), cada um com identidade propria criada, ambos
online e conectados (contador de peers > 0 nos dois).

### Envio 1:1 (iOS) — ponta a ponta
- [ ] No A: NewChatView -> inserir o Peer ID do B -> iniciar conversa (sem erro
      de rede; com peer offline a UI mostra "Falha ao iniciar conversa")
- [ ] No A: digitar e enviar mensagem
- [ ] **No A a mensagem aparece no chat** (persistencia local do remetente —
      hoje falha com peer offline, ver bug abaixo)
- [ ] **No B a mensagem chega** (no chat e/ou notificacao)
- [ ] Input limpo apos enviar

### Envio de grupo — ponta a ponta
- [ ] No A: criar grupo e adicionar o B como membro
- [ ] No A: enviar mensagem no grupo -> aparece no compositor do A
- [ ] **A mensagem persiste no A** (com 1 device a mensagem propria NAO e salva)
- [ ] **No B a mensagem do grupo chega**

### Busca (iOS) — dentro da conversa 1:1
> No iOS a busca vive **dentro** da conversa 1:1 (inacessivel com 1 device); no
> Android a busca e global, coberta pelo flow Maestro `06_busca`.
- [ ] Abrir uma conversa 1:1 com historico de mensagens
- [ ] Abrir a busca dentro da conversa -> digitar um termo -> resultados corretos
- [ ] Busca sem resultado nao trava a tela

### Galeria de midia (iOS) — dentro da conversa 1:1
> No iOS a galeria vive **dentro** da conversa 1:1; no Android e coberta pelo
> flow Maestro `10_media_gallery`.
- [ ] Enviar/receber uma imagem ou video numa conversa 1:1 (2 devices)
- [ ] Abrir a galeria de midia da conversa -> a midia aparece
- [ ] Galeria vazia mostra empty state sem crash

> **Bug de produto conhecido** (ver `e2e/maestro/README.md`): com o peer
> offline o envio **nao persiste a mensagem localmente** (SQLite: tabela
> `messages` vazia) e `GroupChatView.sendMessage` engole o erro sem feedback ao
> usuario. Ao validar os cenarios acima, confirmar se a mensagem propria aparece
> no remetente mesmo antes de o peer estar online.

## iOS

### Basico
- [ ] Login/onboarding concluido sem erros
- [ ] Peer ID exibido e salvo
- [ ] Lista de conversas carrega sem crash

### P2P
- [ ] Enviar mensagem com ambos online (iOS -> outro)
- [ ] Receber mensagem com ambos online (outro -> iOS)
- [ ] Contador de peers > 0 quando conectado

### Store & Forward
- [ ] Enviar mensagem para destinatario offline
- [ ] Destinatario recebe ao voltar online

### Push (APNs)
- [ ] Token APNs gerado no device
- [ ] Token registrado no push server
- [ ] Notificacao chega com app fechado
- [ ] Tap abre conversa correta

### VoIP (audio)
- [ ] Chamada de voz iOS <-> Android
- [ ] Chamada de voz iOS <-> Desktop
- [ ] Audio funciona nos dois sentidos

## Android

### Basico
- [ ] Login/onboarding concluido sem erros
- [ ] Peer ID exibido e salvo
- [ ] Lista de conversas carrega sem crash

### P2P
- [ ] Enviar mensagem com ambos online (Android -> outro)
- [ ] Receber mensagem com ambos online (outro -> Android)
- [ ] Contador de peers > 0 quando conectado

### Store & Forward
- [ ] Enviar mensagem para destinatario offline
- [ ] Destinatario recebe ao voltar online

### Push (FCM)
- [ ] Token FCM gerado
- [ ] Token registrado no push server
- [ ] Notificacao chega com app fechado
- [ ] Tap abre conversa correta

### VoIP (audio)
- [ ] Chamada de voz Android <-> iOS
- [ ] Chamada de voz Android <-> Desktop
- [ ] Audio funciona nos dois sentidos

## Desktop

### Basico
- [ ] Login/onboarding concluido sem erros
- [ ] Peer ID exibido e salvo
- [ ] Lista de conversas carrega sem crash

### P2P
- [ ] Enviar mensagem com ambos online (Desktop -> outro)
- [ ] Receber mensagem com ambos online (outro -> Desktop)
- [ ] Contador de peers > 0 quando conectado

### Store & Forward
- [ ] Enviar mensagem para destinatario offline
- [ ] Destinatario recebe ao voltar online

### VoIP (audio)
- [ ] Chamada de voz Desktop <-> Android
- [ ] Chamada de voz Desktop <-> iOS
- [ ] Audio funciona nos dois sentidos
