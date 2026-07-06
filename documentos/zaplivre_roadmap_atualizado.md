# ZapLivre - Roadmap Atualizado (COM CHAMADAS)

## 🚨 MUDANÇA CRÍTICA NO PLANO

**DESCOBERTA:** Sem chamadas de voz, ZapLivre NÃO vai decolar.

87% dos brasileiros usam WhatsApp para chamadas. Sem "dar um toque", ninguém migra.

**ROADMAP ANTERIOR (ERRADO):**
- Mês 1-2: Setup
- Mês 3-6: Mensagens + Android app
- Mês 7+: "Talvez chamadas"

**ROADMAP NOVO (CORRETO):**
- Mês 1-2: Setup
- Mês 3: Mensagens básicas
- **Mês 4: CHAMADAS DE VOZ** ← PRIORIDADE MÁXIMA
- Mês 5: iOS + Videochamadas
- Mês 6+: Polimento

---

## 📅 TIMELINE REVISADA

### Mês 1-2: Fundação (MANTÉM)
✅ Landing page online
✅ 50-100 beta testers
✅ GitHub público
✅ Identidade visual
✅ Bootstrap nodes funcionando

**Entregável:** zaplivre.app captando emails

---

### Mês 3: Mensagens Básicas
🎯 **Objetivo:** Chat 1:1 funcionando (sem grupo ainda)

**Features:**
- [ ] Enviar/receber mensagens de texto
- [ ] Notificações push
- [ ] Lista de conversas
- [ ] Status online/offline
- [ ] "Digitando..."
- [ ] Ícone de entregue/lido

**Apps:**
- [ ] Android MVP (Kotlin + Compose)
- [ ] Desktop MVP (Tauri)
- [ ] iOS? (pode esperar)

**Infrastructure:**
- [ ] Signaling server (WebSocket)
- [ ] Message store (offline)
- [ ] Push notification service

**Entregável:** 10 beta testers usando para trocar mensagens

---

### Mês 4: CHAMADAS DE VOZ 🔥 (PRIORIDADE MÁXIMA)

🎯 **Objetivo:** "Dar um toque" funcionando perfeitamente

**SEM ISSO, NINGUÉM ADOTA. PONTO.**

#### Semana 1-2: WebRTC Core
- [ ] Signaling server para chamadas
- [ ] WebRTC integration (Android)
- [ ] WebRTC integration (Desktop)
- [ ] TURN/STUN já temos (reusar do chat)
- [ ] Testes de conectividade

#### Semana 3: UI Chamadas
- [ ] Tela de chamada (incoming/outgoing)
- [ ] Botões: atender/recusar/desligar
- [ ] Timer de duração
- [ ] Indicador de qualidade
- [ ] Notificação fullscreen (Android)

#### Semana 4: Polimento + Testes
- [ ] Echo cancellation
- [ ] Noise suppression
- [ ] Adaptive bitrate
- [ ] Funciona em background
- [ ] Funciona com Bluetooth
- [ ] Histórico de chamadas

**Entregável:** Beta testers conseguem fazer chamadas de voz com qualidade comparável ao WhatsApp

**CRÍTICO:** Se chamadas não funcionarem bem, PARA TUDO e conserta antes de continuar.

---

### Mês 5: iOS + Videochamadas

#### iOS App (Semanas 1-2)
- [ ] Swift + SwiftUI
- [ ] Mensagens de texto
- [ ] Chamadas de voz
- [ ] CallKit integration
- [ ] TestFlight beta

#### Videochamadas (Semanas 3-4)
- [ ] Vídeo 1:1 (Android)
- [ ] Vídeo 1:1 (iOS)
- [ ] Vídeo 1:1 (Desktop)
- [ ] Câmera front/back
- [ ] Mute áudio/vídeo

**Entregável:** Paridade de features básicas entre Android/iOS/Desktop

---

### Mês 6: Grupos + Polimento

#### Grupos (Semanas 1-2)
- [ ] Chat em grupo (até 256 pessoas)
- [ ] Admin controls
- [ ] Chamadas em grupo (até 8 pessoas)
- [ ] Deploy SFU server

#### Polimento (Semanas 3-4)
- [ ] Envio de imagens
- [ ] Envio de vídeos
- [ ] Compartilhamento de arquivos
- [ ] Mensagens de voz
- [ ] Reactions

**Entregável:** Feature parity com WhatsApp (básico)

---

## 🎯 MVP para Lançamento Público (6 meses)

### OBRIGATÓRIO (Sem isso não lança):
✅ Mensagens de texto 1:1
✅ **Chamadas de voz 1:1** ← DEAL-BREAKER
✅ Notificações push
✅ Funciona offline (store-and-forward)
✅ Android + Desktop
✅ Grupos de texto (até 256)
✅ Histórico de conversas
✅ Envio de imagens

### IMPORTANTE (Mas pode vir depois):
⚠️ iOS app
⚠️ Videochamadas
⚠️ Chamadas em grupo
⚠️ Mensagens de voz
⚠️ Compartilhamento de arquivos

### NICE TO HAVE (Roadmap pós-lançamento):
🔮 Stories/Status
🔮 Channels (broadcast)
🔮 Polls
🔮 Pagamentos
🔮 Bots/API

---

## 💰 Budget Atualizado (6 meses)

| Item | Custo/mês | 6 meses | Necessidade |
|------|-----------|---------|-------------|
| **VPS (Bootstrap + Signaling)** | R$ 100 | R$ 600 | 🔴 Essential |
| **TURN Server (Chamadas)** | R$ 200 | R$ 1.200 | 🔴 Essential |
| **Message Store (Redis + PG)** | R$ 150 | R$ 900 | 🔴 Essential |
| **SFU Server (Grupos)** | R$ 300 | R$ 1.800 | 🟡 Important |
| **Push Notifications** | R$ 0-50 | R$ 300 | 🔴 Essential |
| **Domain + Email** | R$ 50 | R$ 300 | 🔴 Essential |
| **Anúncios (opcional)** | R$ 500 | R$ 3.000 | 🟢 Optional |
| **TOTAL MÍNIMO** | **R$ 500** | **R$ 3.000** | |
| **TOTAL IDEAL** | **R$ 1.300** | **R$ 8.100** | |

**Nota:** Desenvolvimento é você (custo: tempo)

---

## 📊 Milestones de Validação

### Milestone 1: Mês 3 ✅
- [ ] 50+ beta testers ativos
- [ ] 100+ mensagens trocadas/dia
- [ ] < 5% taxa de bug crítico
- [ ] NPS > 50

**Se não atingir:** Feedback loop, iterar

---

### Milestone 2: Mês 4 (CRÍTICO) 🔥
- [ ] 100% dos beta testers conseguem fazer chamadas
- [ ] Qualidade média > 4.0/5.0 (MOS)
- [ ] < 5% dropped calls
- [ ] Comparação lado-a-lado com WhatsApp = "tão bom quanto"

**Se não atingir:** NÃO avança. Conserta chamadas primeiro.

**TESTE DECISIVO:**
Pergunte aos beta testers: "Você usaria ZapLivre como seu chat principal?"

Se resposta for "Não, porque..." → conserta o "porque"
Se resposta for "Sim!" → avança

---

### Milestone 3: Mês 6
- [ ] 500+ usuários ativos
- [ ] 50+ empresas usando
- [ ] Retenção D7 > 40%
- [ ] NPS > 70

**Se atingir:** Lança público (F-Droid, Play Store)
**Se não:** Mais 2 meses de beta privado

---

## 🚀 Estratégia de Lançamento

### Beta Privado (Mês 1-6)
- 100-500 usuários selecionados
- Foco em distribuidores food service
- Feedback intenso via Discord
- Bugs corrigidos em < 24h

### Soft Launch (Mês 7)
- F-Droid primeiro (Android)
- Post LinkedIn + HN
- "100 vagas para primeiros usuários"
- Não fazer marketing agressivo ainda

### Public Launch (Mês 8-9)
- Google Play Store
- Campanha marketing
- Press release: "WhatsApp brasileiro"
- Anúncios Google: "whatsapp banido"

---

## ⚠️ Riscos Atualizados

### Risco #1: Chamadas não funcionam bem
**Probabilidade:** Média (WebRTC é complexo)
**Impacto:** CATASTRÓFICO (projeto morre)
**Mitigação:**
- Começar chamadas cedo (Mês 4)
- Testes exaustivos com beta testers
- Não lançar até qualidade estar boa

### Risco #2: iOS não pronto no lançamento
**Probabilidade:** Média-Alta
**Impacto:** Médio (Brasil é Android-heavy)
**Mitigação:**
- Lançar Android-first
- iOS vem depois (3-6 meses delay ok)
- Marketing: "Versão iOS em breve"

### Risco #3: Custos de infraestrutura explodem
**Probabilidade:** Baixa (P2P economiza)
**Impacto:** Alto
**Mitigação:**
- 80% chamadas P2P (zero custo)
- Monitorar custos TURN semanalmente
- Tier pago se necessário

---

## 🎯 Decisão GO/NO-GO

**Ao final do Mês 4, perguntar:**

1. Chamadas funcionam bem? (> 4.0/5.0 MOS)
2. Beta testers estão entusiasmados?
3. Algum beta tester substituiu WhatsApp por ZapLivre?

**Se 3x SIM:** Continua full speed 🚀
**Se 2x SIM:** Continua mas com cautela ⚠️
**Se < 2 SIM:** Para, pivota ou cancela ⛔

---

## 💡 LIÇÃO APRENDIDA

**Erro do planejamento anterior:**
- Focou demais em "P2P é legal tecnicamente"
- Esqueceu "usuários precisam de chamadas"

**Correção:**
- Chamadas são P0 (prioridade máxima)
- Tecnologia P2P é meio, não fim
- Foco em paridade de features com WhatsApp

**Nova filosofia:**
> "ZapLivre precisa ser BOM COMO WhatsApp primeiro.
> MELHOR QUE WhatsApp (sem ban, privado) é o diferencial.
> Mas se não for bom, o diferencial não importa."

---

## 📱 Mensagem de Marketing Atualizada

### ANTES (Incompleto):
"ZapLivre: chat sem ban, open source, privado"

**Problema:** "Ok, mas tem chamadas?"

### DEPOIS (Completo):
```
ZAPLIVRE

✅ Mensagens ilimitadas
✅ Chamadas de voz
✅ Videochamadas  
✅ Grupos

MAS SEM:
❌ Ban
❌ Limite
❌ Meta espionando

Tudo que você usa no WhatsApp.
Sem o que você ODEIA no WhatsApp.
```

---

**Este roadmap substitui o anterior.**

**Foco total: Mês 4 = Chamadas funcionando.**

**Tudo é secundário até isso estar pronto.** 🎯📞
