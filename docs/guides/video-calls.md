# Fase 14 - Especificacao tecnica de videochamadas

Objetivo: definir o pipeline de video end-to-end e o escopo do MVP.

## 1) Pipeline (alto nivel)

1. Captura de camera (frame raw)
2. Pre-processamento (resize, fps, colorspace)
3. Encode (VP8 ou VP9)
4. Packetizacao RTP (payload format)
5. Transporte (WebRTC / SRTP)
6. Depacketizacao + decode
7. Render (preview local + remoto)

## 2) Codecs e payload

- Codec inicial: **VP8** (menos carga, suporte amplo).
- Opcional futuro: **VP9**.
- Payload: RTP com frame fragmentation e reassembly.
- Tamanho alvo: 640x360 @ 15-30fps (MVP).

## 3) Sinalizacao

Usar o canal P2P existente para negociar:
- Offer/Answer (SDP)
- ICE candidates
- Preferencia de codec
- Largura de banda / bitrate alvo

## 4) NAT e relay

- ICE com STUN + TURN (coturn).
- Fallback: relay quando NAT simetrico.
- Reuso do fluxo atual de chamadas de voz.

## 5) UI/UX (MVP)

- Tela de chamada com:
  - preview local
  - video remoto
  - botoes mute, camera on/off, hangup
- Indicador de conexao (connecting, connected, failed)

## 6) Telemetria e logs

- Logs de setup (offer/answer, ICE, connected)
- Stats basicos (fps, bitrate, packet loss)

## 7) Criterios de aceite (MVP)

- Video 1:1 em pelo menos 1 plataforma
- Audio continua funcionando em paralelo
- Chamada conecta em rede local e via TURN

## 8) Plano de implementacao (sugestao)

1. Implementar pipeline de captura -> encode -> RTP (1 plataforma).
2. Receber RTP e renderizar remoto.
3. Amarrar sinalizacao no fluxo de call existente.
4. Validar TURN/relay com testes manuais.
