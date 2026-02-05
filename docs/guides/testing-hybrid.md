# Plano de Testes Hibrido (Manual + Automatizado)

Este guia combina scripts/checklists manuais com testes automatizados para validar
o fluxo P2P, push, e funcionalidades criticas antes de release.

## 1) Manual (scripts + checklist)

### Push (iOS + Android)
- Script APNs: `scripts/test-apns.sh`
- Script FCM: `scripts/test-fcm.sh`
- Checklist: `docs/guides/push-checklist.md`

### Fluxo base P2P (2 dispositivos)
- Onboarding em 2 dispositivos
- Troca de mensagens P2P com app em foreground
- Validar "connected peers" > 0
- Testar offline: enviar mensagem com destinatario offline e verificar entrega quando voltar

### VoIP (audio)
- Chamada Android <-> Desktop
- Chamada Android <-> iOS
- Validar permissao de microfone e troca de audio

## 2) Automatizados (base atual)

### Core (Rust)
```bash
cd core
cargo test --workspace
```

### Integrais (ignorados)
```bash
cd core
cargo test --test voip_integration -- --ignored
```

### Push server (Rust)
```bash
cd server/push
cargo test
```

## 3) Criterios de aceite

- Push entrega e navega para conversa correta em iOS/Android
- Mensagens P2P entregues com ambos online e offline (store)
- VoIP audio funciona em pelo menos 2 plataformas
