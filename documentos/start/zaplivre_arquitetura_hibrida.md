# ZapLivre - Arquitetura HÍBRIDA (P2P + Servidor)

## 🚨 ESCLARECIMENTO CRÍTICO

**ZapLivre NÃO é P2P puro. É HÍBRIDO.**

```
❌ ERRADO: "ZapLivre é completamente descentralizado, sem servidores"
✅ CORRETO: "ZapLivre usa P2P quando possível, servidor quando necessário"
```

## 🎯 Por que HÍBRIDO?

### P2P Puro (tipo Briar, Jami) - Problemas:

❌ **Mensagens offline não funcionam**
- Se destinatário offline → mensagem perdida
- Precisa ambos online simultaneamente
- UX horrível para usuário casual

❌ **Discovery é lento**
- Encontrar peer pode demorar minutos
- Depende de DHT convergir
- Primeira conexão é dolorosa

❌ **NAT traversal falha ~20% do tempo**
- Alguns NATs são "simétricos" (bloqueiam tudo)
- CGN (Carrier-Grade NAT) não funciona
- Firewalls corporativos bloqueiam

❌ **Incompatível com expectativa de usuário**
- Usuário espera mensagem chegar INSTANTANEAMENTE
- Mesmo se destinatário offline
- Mesmo se ambos atrás de firewall

### Híbrido (ZapLivre) - Solução:

✅ **Melhor dos 2 mundos:**
- P2P quando possível (80% dos casos) → privacidade máxima, zero custo
- Servidor quando necessário (20% dos casos) → confiabilidade

✅ **UX igual ao WhatsApp:**
- Mensagem sempre chega
- Funciona offline
- Instantâneo

---

## 📐 ARQUITETURA HÍBRIDA - COMPLETA

```
┌─────────────────────────────────────────────────────────────┐
│                  ZAPLIVRE HYBRID ARCHITECTURE                 │
└─────────────────────────────────────────────────────────────┘

SCENARIO 1: P2P Direto (80% dos casos)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

[Alice Mobile]                          [Bob Desktop]
     │                                        │
     ├─ 1. Peer discovery via DHT ────────────┤
     │   (Bootstrap node ajuda)               │
     │                                        │
     ├─ 2. NAT traversal (STUN)               │
     │   Testa conexão direta                 │
     │                                        │
     │◀═══ 3. P2P Connection ════════════════▶│
     │      (criptografado E2E)               │
     │      (zero custo servidor)             │
     │      (latência mínima ~50ms)           │
     │                                        │
     ├─ Mensagem vai DIRETO ──────────────────▶│
     │  (não passa por servidor)              │
     
VANTAGENS:
✅ Privacidade total (servidor nunca vê conteúdo)
✅ Zero custo operacional
✅ Latência mínima
✅ Bandwidth grátis

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

SCENARIO 2: Relay (NAT Simétrico) (~15% dos casos)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

[Alice Mobile]         [TURN Relay]         [Bob Desktop]
  (NAT Simétrico)                           (Firewall Corp)
     │                      │                     │
     ├─ P2P tentado ────────┼─────────────────────┤
     │  ❌ FALHA            │                     │
     │  (NAT bloqueia)      │                     │
     │                      │                     │
     ├─ Fallback TURN ──────▶                    │
     │                      │                     │
     │ Mensagem ────────────▶ Relay ─────────────▶│
     │  (ainda E2E encrypt) │ (só roteia)        │
     │                      │                     │

DETALHE IMPORTANTE:
- Relay NÃO descriptografa (ainda é E2E)
- Relay só roteia packets criptografados
- Mais latência (~150-200ms)
- Custa bandwidth no servidor

QUANDO ACONTECE:
- NAT simétrico (Vivo, Tim 4G às vezes)
- Firewall corporativo muito restritivo
- Ambos peers atrás de CGN
- VPN corporativa

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

SCENARIO 3: Destinatário Offline (~5% dos casos)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

[Alice Mobile]         [Message Store]      [Bob Desktop]
     │                      │                  (OFFLINE)
     │                      │                     
     ├─ Tenta P2P ──────────┼─────────────────X
     │  ❌ Bob offline      │
     │                      │
     ├─ Store & Forward ────▶
     │  Salva no PostgreSQL │
     │  TTL: 14 dias        │
     │                      │
     │                      │     [Bob fica online]
     │                      │              │
     │                      │◀─ Conecta ───┤
     │                      │              │
     │                      ├─ Entrega ────▶│
     │                      │  mensagens    │
     │◀─ ACK ───────────────┤  pendentes    │
     │  "Entregue"          │              │

DETALHE IMPORTANTE:
- Mensagem salva CRIPTOGRAFADA (servidor não lê)
- Server só sabe: "alice → bob, timestamp, size"
- Após entrega → deletada do servidor
- Se não entregue em 14 dias → expirada e deletada

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

SCENARIO 4: Multi-Device Sync
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

[Alice Mobile]    [Alice Desktop]    [Alice Tablet]
     │                  │                  │
     │                  │                  │
     ├─ Nova mensagem ─▶│                  │
     │  (CRDT)          │                  │
     │                  │                  │
     │                  ├─ Sync ───────────▶│
     │                  │  (P2P direto)    │
     │                  │  (ou via relay)  │
     │                  │                  │
     │◀── Sync ─────────┤                  │
     │   (bidirectional)│                  │

DETALHE IMPORTANTE:
- Devices do MESMO usuário sincronizam via P2P
- CRDTs garantem consistência
- Funciona offline (eventual consistency)
- Servidor NÃO está no meio (privacy)

Mas se device offline muito tempo:
     │
     ├─ Sync state ────────▶ [Sync Server]
     │  (opcional backup)   (CRDT state)
     │                      (criptografado)

```

---

## 🔀 FLUXO DE DECISÃO: P2P ou Servidor?

```
┌─────────────────────────────────────┐
│  Alice quer enviar mensagem pra Bob │
└───────────────┬─────────────────────┘
                │
                ▼
        ┌───────────────┐
        │ Bob está      │
        │ online?       │
        └───┬───────────┘
            │
    ┌───────┴──────┐
    │              │
   SIM            NÃO
    │              │
    ▼              ▼
┌────────┐    ┌──────────────────┐
│Tentativa│    │ Store & Forward  │
│P2P      │    │ (PostgreSQL)     │
│direto   │    │ TTL: 14 dias     │
└────┬───┘    └──────────────────┘
     │
     ▼
┌──────────┐
│P2P       │
│funciona? │
└────┬─────┘
     │
  ┌──┴──┐
  │     │
 SIM   NÃO
  │     │
  │     ▼
  │  ┌──────────┐
  │  │ TURN     │
  │  │ Relay    │
  │  └──────────┘
  │
  ▼
┌────────────────┐
│ Mensagem       │
│ entregue!      │
└────────────────┘
```

---

## 🏗️ COMPONENTES DO SISTEMA

### Client (Local - Device do Usuário)

```
┌────────────────────────────────────┐
│         ZapLivre Client             │
├────────────────────────────────────┤
│                                    │
│  ┌──────────────────────────────┐ │
│  │  P2P Stack (libp2p)          │ │
│  │  • Kademlia DHT              │ │
│  │  • GossipSub (groups)        │ │
│  │  • Circuit Relay             │ │
│  │  • DCUtR (hole-punch)        │ │
│  └──────────────────────────────┘ │
│                                    │
│  ┌──────────────────────────────┐ │
│  │  Local Storage (SQLite)      │ │
│  │  • Messages                  │ │
│  │  • Contacts                  │ │
│  │  • Keys                      │ │
│  │  • Settings                  │ │
│  └──────────────────────────────┘ │
│                                    │
│  ┌──────────────────────────────┐ │
│  │  Crypto (Signal Protocol)    │ │
│  │  • Double Ratchet            │ │
│  │  • X3DH                      │ │
│  │  • Sender Keys (groups)      │ │
│  └──────────────────────────────┘ │
│                                    │
│  ┌──────────────────────────────┐ │
│  │  VoIP (WebRTC)               │ │
│  │  • Peer Connection           │ │
│  │  • ICE/STUN                  │ │
│  │  • Opus codec                │ │
│  └──────────────────────────────┘ │
│                                    │
└────────────────────────────────────┘
```

### Server-Side Components (Self-Hosted)

```
┌────────────────────────────────────┐
│    1. Bootstrap Nodes              │
│    (Rust + libp2p)                 │
├────────────────────────────────────┤
│  Função:                           │
│  • Peer discovery inicial          │
│  • DHT seeding                     │
│  • NAT type detection              │
│                                    │
│  Escala:                           │
│  • 3-5 nodes geograficamente       │
│    distribuídos                    │
│  • Brazil, US, EU                  │
│                                    │
│  Custo:                            │
│  • ~R$ 50/mês cada (VPS pequeno)   │
│  • Total: R$ 150-250/mês           │
└────────────────────────────────────┘

┌────────────────────────────────────┐
│    2. Message Store                │
│    (PostgreSQL + Redis)            │
├────────────────────────────────────┤
│  Função:                           │
│  • Store-and-forward (offline)     │
│  • Message queue (delivery)        │
│  • Presence (online/offline)       │
│                                    │
│  Dados armazenados:                │
│  • Encrypted message blobs         │
│  • Recipient ID                    │
│  • Timestamp, TTL                  │
│  • Delivery status                 │
│                                    │
│  Segurança:                        │
│  • Server NÃO tem chaves           │
│  • Não pode ler conteúdo           │
│  • Apenas metadata mínima          │
│                                    │
│  Custo:                            │
│  • R$ 100-200/mês (1000 users)     │
└────────────────────────────────────┘

┌────────────────────────────────────┐
│    3. TURN Relay                   │
│    (coturn)                        │
├────────────────────────────────────┤
│  Função:                           │
│  • NAT traversal fallback          │
│  • Relay WebRTC (messages + calls) │
│                                    │
│  Quando usado:                     │
│  • NAT simétrico (~15% casos)      │
│  • Firewall muito restritivo       │
│  • VPN corporativa                 │
│                                    │
│  Importante:                       │
│  • Relay NÃO descriptografa        │
│  • Apenas roteia packets           │
│  • E2E preservado                  │
│                                    │
│  Custo:                            │
│  • R$ 150-300/mês (bandwidth)      │
└────────────────────────────────────┘

┌────────────────────────────────────┐
│    4. SFU Server                   │
│    (mediasoup)                     │
├────────────────────────────────────┤
│  Função:                           │
│  • Group calls (voz/vídeo)         │
│  • Selective forwarding            │
│                                    │
│  Quando usado:                     │
│  • Chamadas em grupo (>2 pessoas)  │
│  • Otimiza bandwidth client        │
│                                    │
│  Custo:                            │
│  • R$ 200-400/mês                  │
│                                    │
│  Nota:                             │
│  • Opcional para MVP               │
│  • Grupos pequenos podem usar mesh │
└────────────────────────────────────┘

┌────────────────────────────────────┐
│    5. Push Notification            │
│    (FCM + APNs)                    │
├────────────────────────────────────┤
│  Função:                           │
│  • Acorda app quando mensagem      │
│  • Funciona com device em sleep    │
│                                    │
│  Providers:                        │
│  • FCM (Android) - Google          │
│  • APNs (iOS) - Apple              │
│  • UnifiedPush (alternativa FOSS)  │
│                                    │
│  Custo:                            │
│  • FCM: grátis                     │
│  • APNs: grátis                    │
│  • Hosting push server: R$ 50/mês  │
└────────────────────────────────────┘
```

---

## 💰 CUSTOS OPERACIONAIS (1000 usuários ativos)

```
┌─────────────────────────┬──────────┬────────────────┐
│ Componente              │ Custo/mês│ P2P Offset     │
├─────────────────────────┼──────────┼────────────────┤
│ Bootstrap Nodes (3x)    │ R$ 150   │ N/A (required) │
│ Message Store (PG+Redis)│ R$ 150   │ -80% (P2P)     │
│ TURN Relay              │ R$ 250   │ -85% (P2P)     │
│ SFU (Group calls)       │ R$ 300   │ -70% (P2P)     │
│ Push Notifications      │ R$ 50    │ N/A (required) │
├─────────────────────────┼──────────┼────────────────┤
│ TOTAL                   │ R$ 900   │                │
└─────────────────────────┴──────────┴────────────────┘

SEM P2P (100% servidor, tipo WhatsApp):
• Bandwidth: R$ 2.000/mês
• Processing: R$ 1.500/mês  
• Storage: R$ 500/mês
• TOTAL: ~R$ 4.000/mês

ECONOMIA P2P: ~75% (R$ 3.100/mês economizados)
```

**Por que híbrido é mais barato:**
- 80% mensagens vão P2P direto (zero custo)
- 15% usam relay (custo moderado)
- 5% store-and-forward (custo baixo)

---

## 🔐 SEGURANÇA E PRIVACIDADE

### O que o servidor VÊ:

```sql
-- Message Store
SELECT 
    recipient_id,        -- "bob123" (sabe pra quem)
    sender_id,           -- "alice456" (sabe de quem)
    encrypted_payload,   -- [blob binário] (NÃO LÊ)
    timestamp,           -- "2025-01-19 14:23" (quando)
    size_bytes           -- 1024 (tamanho)
FROM offline_messages;
```

**Servidor NÃO sabe:**
- ❌ Conteúdo da mensagem (criptografado)
- ❌ Subject/assunto
- ❌ Mídia (imagem/vídeo)
- ❌ Localização

**Servidor sabe (metadata mínima):**
- ✅ Alice → Bob (quem fala com quem)
- ✅ Timestamp (quando)
- ✅ Tamanho (quantos bytes)

### Comparação:

**WhatsApp (centralizado):**
- ✅ Conteúdo E2E encrypted (não leem)
- ❌ Metadata completa (quem, quando, onde, quanto)
- ❌ Todos dados passam pelos servers
- ❌ Facebook correlaciona com perfil

**ZapLivre (híbrido):**
- ✅ Conteúdo E2E encrypted
- ✅ Metadata mínima (só offline delivery)
- ✅ 80% mensagens nem passam por servidor (P2P direto)
- ✅ Sem perfil correlacionado

**Briar/Jami (P2P puro):**
- ✅ Zero metadata (servidor nem existe)
- ❌ MAS não funciona offline
- ❌ UX péssima

---

## 📊 ESTATÍSTICAS ESPERADAS

### Distribuição de Tráfego:

```
┌─────────────────────────────────────┐
│                                     │
│  P2P Direto:        ████████ 80%   │
│  (zero custo)                       │
│                                     │
│  TURN Relay:        ███ 15%        │
│  (custo moderado)                   │
│                                     │
│  Store & Forward:   █ 5%           │
│  (custo baixo)                      │
│                                     │
└─────────────────────────────────────┘
```

### Chamadas de Voz:

```
P2P Direto:         ███████████ 85%
(latência ~50ms)

TURN Relay:         ███ 15%
(latência ~200ms)
```

### Multi-Device Sync:

```
P2P entre devices:  ████████████ 95%
(mesmo usuário)

Server fallback:    █ 5%
(um device offline muito tempo)
```

---

## 🎯 POR QUE HÍBRIDO É MELHOR

### vs P2P Puro (Briar, Jami):

| Aspecto | P2P Puro | Híbrido (ZapLivre) |
|---------|----------|-------------------|
| **Privacidade** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ |
| **Confiabilidade** | ⭐⭐ | ⭐⭐⭐⭐⭐ |
| **UX** | ⭐⭐ | ⭐⭐⭐⭐⭐ |
| **Offline** | ❌ Não funciona | ✅ Funciona |
| **Adoção** | ⭐ Muito baixa | ⭐⭐⭐⭐ Possível |

### vs Centralizado (WhatsApp, Telegram):

| Aspecto | Centralizado | Híbrido (ZapLivre) |
|---------|--------------|-------------------|
| **Privacidade** | ⭐⭐⭐ | ⭐⭐⭐⭐ |
| **Confiabilidade** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| **Custo Operacional** | ⭐⭐ Alto | ⭐⭐⭐⭐ Baixo |
| **Censura** | ⭐⭐ Vulnerável | ⭐⭐⭐⭐ Resistente |
| **Self-hosting** | ❌ Impossível | ✅ Possível |

**Conclusão:** Híbrido pega o melhor dos 2 mundos.

---

## 🚀 MARKETING: Como Comunicar

### ❌ ERRADO:

"ZapLivre é completamente descentralizado, P2P puro, sem nenhum servidor"

**Problemas:**
- Promessa impossível de cumprir (precisa store-and-forward)
- Usuário pergunta: "e se meu amigo tá offline?"
- Levanta expectativas irreais

### ✅ CORRETO:

**Mensagem curta:**
"ZapLivre: chat privado sem ban. Suas mensagens não passam pela Meta."

**Mensagem técnica (devs):**
"Arquitetura híbrida P2P + servidor. 80% do tráfego vai direto peer-to-peer (privacidade + economia). Servidores são fallback para confiabilidade, mas não leem seu conteúdo."

**Mensagem comparativa:**
"Como WhatsApp:
✅ Funciona offline
✅ Chamadas de voz
✅ Instantâneo

MELHOR que WhatsApp:
✅ Sem ban (seu servidor ou self-hosted)
✅ Menos metadata (80% mensagens P2P)
✅ Open source (auditável)"

---

## 📋 CHECKLIST DE IMPLEMENTAÇÃO

### Fase 1: P2P Core (Mês 3)
- [ ] libp2p setup (TCP + QUIC transports)
- [ ] Kademlia DHT (peer discovery)
- [ ] 2 peers conectam P2P direto
- [ ] Mensagem vai peer-to-peer
- [ ] **SEM servidor ainda (exceto bootstrap)**

### Fase 2: Server Fallback (Mês 4)
- [ ] Bootstrap nodes (3x geograficamente)
- [ ] TURN relay setup (coturn)
- [ ] Detecta quando P2P falha
- [ ] Fallback automático para relay
- [ ] Teste: NAT simétrico funciona

### Fase 3: Store & Forward (Mês 4)
- [ ] PostgreSQL setup
- [ ] Redis (presence + queue)
- [ ] Mensagem offline salva no DB
- [ ] Entrega quando recipient fica online
- [ ] Auto-delete após entrega ou 14 dias
- [ ] Teste: enviar mensagem pra offline funciona

### Fase 4: Multi-Device (Mês 5)
- [ ] CRDTs (Automerge)
- [ ] Sync entre devices do mesmo usuário (P2P)
- [ ] Server backup de sync state (opcional)
- [ ] Teste: 3 devices sincronizam

### Fase 5: VoIP (Mês 4-5)
- [ ] WebRTC P2P (chamadas diretas)
- [ ] TURN fallback (NAT simétrico)
- [ ] Teste: chamada funciona em 100% cenários

---

## 🎯 RESUMO EXECUTIVO

### ZapLivre é HÍBRIDO porque:

1. **Confiabilidade importa**
   - Usuário espera mensagem chegar sempre
   - Offline precisa funcionar
   - NAT travado precisa ter solução

2. **Privacidade via arquitetura**
   - 80% tráfego P2P (servidor não vê)
   - 20% via servidor mas E2E encrypted
   - Metadata mínima (só delivery offline)

3. **Custo otimizado**
   - P2P elimina 75% do custo vs centralizado
   - Servidor só pra fallback e offline
   - Escalável economicamente

4. **Compatível com expectativas**
   - UX igual WhatsApp (instantâneo, confiável)
   - Privacidade melhor que WhatsApp
   - Self-hosting possível

**Não é P2P vs Servidor.**

**É P2P E Servidor trabalhando juntos.** 🎯

---

## 📞 COMUNICAÇÃO INTERNA

### Para Desenvolvedores:
"Arquitetura híbrida. Tenta P2P primeiro, server fallback. 80/15/5 split."

### Para Investidores:
"Custos 75% menores que centralizado via P2P otimizado. Servers só pra confiabilidade."

### Para Usuários:
"Privado, sem ban, funciona sempre. Simples assim."

### Para Reguladores/Jurídico:
"Self-hosted, dados no Brasil (LGPD), E2E encrypted, metadata mínima."

---

**ESTE documento substitui qualquer ambiguidade anterior.**

**ZapLivre = HÍBRIDO P2P + Servidor.** ✅
