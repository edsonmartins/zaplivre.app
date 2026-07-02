# Testes E2E de UI — Maestro (Android + iOS)

Flows black-box com [Maestro](https://maestro.mobile.dev): rodam contra o app
**real** em emulador/simulador, sem nenhuma alteração no código do app —
escolhido porque o `MePassaClientWrapper`/`MePassaCore` são singletons FFI
sem injeção de dependência (mockar exigiria refatoração).

## Setup

```bash
# Instalar o Maestro
curl -Ls https://get.maestro.mobile.dev | bash

# Android: emulador rodando + APK instalado
cd android && ./gradlew installDebug

# iOS: simulador rodando + app instalado
cd ios && ./build-all.sh
```

## Rodando

```bash
# Todos os flows de uma plataforma
maestro test e2e/maestro/android/
maestro test e2e/maestro/ios/

# Um flow específico
maestro test e2e/maestro/android/01_onboarding_criar.yml
```

## Cobertura dos flows

| Flow | O que valida | Regressão que pegaria |
|---|---|---|
| 01_onboarding_criar | Primeira execução → criar identidade → chegar em Conversas | IDN-01 (corrida do auto-init) |
| 02_navegacao_telas | Toda tela alcançável pela navegação (anti-órfã) | AND-13 (Settings/Profile órfãs) |
| 03_enviar_mensagem | Digitar + enviar → mensagem aparece no chat | fiação UI→FFI de envio |
| 04_backup_identidade | Settings → exportar backup → conteúdo Base64 visível | backup inacessível |
| 05_criar_grupo | Criar grupo → grupo aparece na lista | IOS-10 (lista mockada vazia) |

## Flows que exigem DOIS dispositivos (rodar manualmente por enquanto)

- **Receber mensagem**: enviar do device A, verificar notificação/chat no B
- **Chamada recebida**: ligar do A, verificar tela de IncomingCall no B
  (a regressão do "callee cego" — no desktop isso JÁ é coberto pelo teste
  automatizado `App.incoming-call.test.tsx`)

Roteiro completo multi-dispositivo: `docs/guides/testing-manual.md`.

## Notas

- Os textos usados nos flows são os literais atuais da UI (pt-BR/en misto);
  se a UI mudar textos, atualizar os flows.
- `clearState: true` no onboarding garante primeira execução limpa.
- iOS: os flows assumem o bundle id `app.mepassa.ios`.
