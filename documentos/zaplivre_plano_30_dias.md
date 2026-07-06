# ZapLivre - Plano de Execução 30 Dias (Domínio Registrado ✅)

## 🎯 Objetivo: Lançar MVP e conseguir primeiros 100 beta testers

**Domínio confirmado:** zaplivre.app ✅

---

## 📅 SEMANA 1: Fundação e Validação Legal

### Dia 1-2: Proteção de Marca
- [ ] **Consulta jurídica express** (prioridade máxima)
  - Advogado especialista em PI/marcas
  - Questão: "ZapLivre pode ser processado pela Meta/WhatsApp?"
  - Custo estimado: R$ 500-1.000
  - Se resposta for "risco alto" → pivot para "Negocia"
  
- [ ] **Busca prévia INPI** (você mesmo pode fazer)
  - Acesse: https://busca.inpi.gov.br/pePI/servlet/MarcasServletController
  - Buscar: "ZapLivre", "Zap Livre", variações
  - Verificar conflitos em classe 38 (telecomunicações) e 42 (software)

- [ ] **Registro de marca** (iniciar processo)
  - Se jurídico der OK, protocolar registro no INPI
  - Classe 38: Serviços de telecomunicações
  - Classe 42: Software como serviço
  - Custo: ~R$ 500 (processo pode levar 2-3 anos mas prioridade é protocolar)

### Dia 3-4: Domínios e Identidade

- [ ] **Registrar domínios complementares**
  - zaplivre.com.br (se disponível e preço razoável)
  - zaplivre.com (se disponível)
  - Alternativa: redirecionar para zaplivre.app
  
- [ ] **Registrar redes sociais**
  - Instagram: @zaplivre
  - Twitter/X: @zaplivre
  - Facebook: /zaplivre
  - LinkedIn: /company/zaplivre
  - YouTube: /zaplivre
  - TikTok: @zaplivre (para futuro)

- [ ] **Email profissional**
  - Configurar Google Workspace ou Zoho
  - contato@zaplivre.app
  - suporte@zaplivre.app
  - comercial@zaplivre.app

### Dia 5-7: Setup GitHub e Infraestrutura

- [ ] **Criar organização GitHub**
  - github.com/zaplivre (ou github.com/edsonmartins/zaplivre.app)
  - Licença: AGPL v3.0
  - README.md inicial
  - CODE_OF_CONDUCT.md
  - CONTRIBUTING.md

- [ ] **Estrutura de repositório**
  ```
  zaplivre/
  ├── core/           # Rust library (omega-core renomeado)
  ├── android/        # Kotlin app
  ├── desktop/        # Tauri app
  ├── server/         # Infrastructure
  │   ├── discovery/
  │   ├── store/
  │   └── relay/
  └── docs/           # Documentation
  ```

- [ ] **Setup infraestrutura dev**
  - Copiar docker-compose.yml dos documentos anteriores
  - Provisionar VPS para bootstrap node
    - Recomendação: Hetzner Germany (barato, confiável)
    - VPS CX11: €4.15/mês (~R$ 25/mês)
  - Configurar DNS para zaplivre.app

---

## 📅 SEMANA 2: Landing Page e Captação de Leads

### Dia 8-10: Landing Page "Coming Soon"

- [ ] **Design landing page**
  - Ferramenta: Figma (grátis) ou direto em código
  - Seções:
    1. Hero: "WhatsApp baniu sua empresa? Conheça ZapLivre"
    2. Problema: Por que empresas estão migrando
    3. Solução: O que é ZapLivre
    4. CTA: "Quero ser beta tester"
    5. Footer: Social + contato

- [ ] **Desenvolver landing page**
  - Stack sugerido: Next.js + TailwindCSS + Vercel (grátis)
  - Ou: HTML estático + Bootstrap (mais simples)
  - Formulário: Typeform ou Google Forms
  - Analytics: Google Analytics 4

- [ ] **Copy da landing page**
  ```
  ═══════════════════════════════════════
  
  🚨 Seu WhatsApp foi banido?
  
  Você não está sozinho.
  Meta bane 2 milhões de contas por dia.
  
  ═══════════════════════════════════════
  
  CONHEÇA ZAPLIVRE
  
  ✅ Nunca te banimos
  ✅ Bots e automação livres  
  ✅ Seus dados no Brasil
  ✅ Open source e gratuito
  
  ═══════════════════════════════════════
  
  [QUERO SER BETA TESTER]
  
  Vagas limitadas: 100 empresas
  Lançamento: Março 2026
  
  ═══════════════════════════════════════
  ```

### Dia 11-12: SEO e Content

- [ ] **Setup SEO básico**
  - Title: "ZapLivre - WhatsApp para empresas sem medo de ban"
  - Description: "Chat profissional com bots, automação e integrações. Nunca te banimos. Open source, LGPD compliance, dados no Brasil."
  - Keywords: whatsapp empresas, alternativa whatsapp business, chat sem ban
  - Open Graph tags para redes sociais

- [ ] **Criar primeiro conteúdo**
  - Blog post: "5 motivos para sua empresa ser banida do WhatsApp"
  - Publicar no Medium + LinkedIn
  - Direcionar para zaplivre.app

### Dia 13-14: Validação com Mercado

- [ ] **Pesquisa com distribuidores** (20-30 empresas)
  - Ligar para clientes VendaX.ai
  - Perguntas:
    1. "Já foi banido do WhatsApp alguma vez?"
    2. "Usa automação/bots para vendas?"
    3. "Pagaria R$ 50-100/mês por alternativa sem ban?"
    4. "O nome ZapLivre te atrai?"
  - Objetivo: 50+ emails para beta

- [ ] **Análise de competidores**
  - Telegram Business
  - Signal
  - Outras alternativas BR
  - Diferenciais de cada
  - Onde ZapLivre é superior

---

## 📅 SEMANA 3: Desenvolvimento Core MVP

### Dia 15-17: ZapLivre Core (Rust)

- [ ] **Renomear omega-core → zaplivre-core**
  - Atualizar Cargo.toml
  - Namespaces: `zaplivre_core`
  - Pacote: `zaplivre-core`

- [ ] **Implementar módulos base**
  - ✅ Identity (keypair Ed25519)
  - ✅ Crypto (Signal Protocol wrapper)
  - ✅ Storage (SQLite básico)
  - ✅ Network (libp2p conexão P2P simples)

- [ ] **CLI tool básico**
  ```bash
  zaplivre-cli init          # Gerar keypair
  zaplivre-cli id            # Mostrar peer ID
  zaplivre-cli connect <peer_id>  # Conectar a peer
  zaplivre-cli send <peer_id> <message>  # Enviar mensagem
  ```

### Dia 18-19: Testes Locais

- [ ] **Testes de conectividade**
  - 2 instâncias do CLI em localhost
  - Enviar mensagem P2P direta
  - Verificar criptografia E2E

- [ ] **Deploy bootstrap node**
  - Configurar VPS Hetzner
  - Deploy omega-discovery (renomear para zaplivre-discovery)
  - Testar descoberta de peers via DHT

### Dia 20-21: Documentação Técnica

- [ ] **README técnico**
  - Arquitetura P2P híbrida
  - Como funciona E2E encryption
  - Diferenças vs WhatsApp/Telegram

- [ ] **API documentation**
  - Rust docs: `cargo doc --open`
  - Publicar em docs.zaplivre.app

---

## 📅 SEMANA 4: Marketing e Pré-lançamento

### Dia 22-24: Campanha Beta

- [ ] **Email para leads captados**
  ```
  Assunto: Você está entre os 100 primeiros do ZapLivre
  
  Olá [Nome],
  
  Obrigado por se inscrever no ZapLivre!
  
  Temos novidades:
  ✅ Código open source disponível (GitHub)
  ✅ Primeira versão funcionando (CLI)
  ✅ Bootstrap node rodando
  
  Próximas semanas: app Android alpha
  
  Quer contribuir? github.com/zaplivre
  
  Abraço,
  Edson - Fundador ZapLivre
  ```

- [ ] **Post no LinkedIn anunciando**
  - "Lancei ZapLivre: WhatsApp open source para empresas"
  - Compartilhar arquitetura técnica
  - Link para GitHub
  - CTA: contribuidores bem-vindos

- [ ] **Primeiros anúncios Google**
  - Budget: R$ 500-1000 (teste)
  - Keywords: "whatsapp banido", "alternativa whatsapp empresas"
  - Landing page: zaplivre.app
  - Objetivo: 100+ emails beta

### Dia 25-27: Comunidade

- [ ] **Criar Discord/Slack**
  - Canais:
    - #geral
    - #desenvolvimento
    - #beta-testers
    - #suporte
    - #ideias

- [ ] **Convidar primeiros contributors**
  - Desenvolvedores Rust conhecidos
  - Estudantes universitários (USP, UNICAMP, etc)
  - Comunidade Rust Brasil

### Dia 28-30: Preparação Android Alpha

- [ ] **Setup projeto Android**
  - Kotlin + Jetpack Compose
  - Integração com zaplivre-core via FFI
  - UI básica: lista de conversas + chat

- [ ] **Design mockups**
  - Figma: telas principais
  - Inspiração: Telegram + Signal
  - Identidade visual ZapLivre

- [ ] **Roadmap público**
  - GitHub Projects
  - Issues marcadas "good first issue"
  - Milestones claros

---

## 🎨 Identidade Visual ZapLivre

### Logo Conceitual

**Opção 1: Raio Livre**
```
    ⚡
   ╱ ╲   ZAPLIVRE
  ╱___╲  ━━━━━━━━
         Venda sem medo
```
Conceito: Raio (Zap = velocidade) quebrando correntes (Livre)

**Opção 2: Balão com Asa**
```
  ┌─┐
  │💬│▶  ZAPLIVRE
  └─┘    ━━━━━━━━
```
Conceito: Mensagem (balão) com liberdade (asa)

**Opção 3: ZL Minimalista**
```
┏━━┓
┃ZL┃  ZAPLIVRE
┗━━┛  Livre pra vender
```

### Paleta de Cores

**Opção 1: Bandeira Brasil (patriótico)**
```
#009B3A - Verde Bandeira (primary)
#FEDD00 - Amarelo Bandeira (accent)
#002776 - Azul Bandeira (secondary)
```

**Opção 2: Tech Moderno (minha recomendação)**
```
#00D9FF - Ciano elétrico (primary) - energético, tech
#7C3AED - Roxo vibrante (secondary) - premium, confiável
#10B981 - Verde sucesso (accent) - positivo, crescimento
#1F2937 - Cinza escuro (text)
#F9FAFB - Branco quente (background)
```

**Opção 3: Confiança Corporate**
```
#0066CC - Azul confiança (primary)
#00AA55 - Verde sucesso (secondary)
#FF6600 - Laranja energia (accent)
```

**Minha recomendação: Opção 2 (Tech Moderno)**
- Destaca de WhatsApp (verde) e Telegram (azul)
- Soa inovador, não corporativo chato
- Funciona bem em dark mode

---

## 💰 Budget Estimado (30 dias)

| Item | Custo | Necessidade |
|------|-------|-------------|
| **Consultoria jurídica** | R$ 500-1.000 | 🔴 Essencial |
| **Registro INPI** | R$ 500 | 🟡 Importante |
| **Domínios (.com.br, .com)** | R$ 100-300 | 🟢 Opcional |
| **VPS Hetzner (bootstrap)** | R$ 25/mês | 🔴 Essencial |
| **Google Workspace** | R$ 30/mês | 🟡 Importante |
| **Anúncios Google (teste)** | R$ 500-1.000 | 🟢 Opcional |
| **Design logo (Fiverr)** | R$ 100-300 | 🟢 Opcional |
| **TOTAL MÍNIMO** | **R$ 1.055** | |
| **TOTAL IDEAL** | **R$ 2.255** | |

**Nota:** Desenvolvimento é você (zero custo monetário, alto custo de tempo)

---

## 📊 KPIs - 30 Dias

### Objetivos Mínimos (Sucesso):
- [ ] 50+ emails beta testers
- [ ] GitHub: 20+ stars
- [ ] CLI funcionando (2 peers comunicando)
- [ ] Bootstrap node estável (99% uptime)
- [ ] Landing page: 500+ visitantes únicos

### Objetivos Ideais (Overachieve):
- [ ] 100+ emails beta testers
- [ ] GitHub: 50+ stars, 3+ contributors
- [ ] Alpha Android rodando (mesmo que bugado)
- [ ] Primeira empresa testando em produção
- [ ] 1.000+ visitantes landing page

---

## 🚨 Riscos e Mitigações

### Risco 1: Meta processa "ZapLivre"
**Probabilidade:** Baixa-Média
**Impacto:** Alto (rebrand completo)
**Mitigação:**
- Consultoria jurídica ASAP (Dia 1-2)
- Se risco confirmado: pivot para "Negocia" imediatamente
- Já ter negocia.app registrado como backup

### Risco 2: Ninguém se inscreve (demanda inexistente)
**Probabilidade:** Baixa (sabemos que empresas estão sofrendo)
**Impacto:** Médio
**Mitigação:**
- Pesquisa com 20+ distribuidores (validação real)
- Se <20 emails em 2 semanas: mudar mensagem/canais

### Risco 3: Desenvolvimento atrasa
**Probabilidade:** Alta (todo projeto de software atrasa)
**Impacto:** Médio
**Mitigação:**
- Scope mínimo: apenas CLI + landing page nos primeiros 30 dias
- Android alpha pode deslizar para 60-90 dias

### Risco 4: Sem budget para marketing
**Probabilidade:** Baixa (você tem VendaX.ai)
**Impacto:** Baixo
**Mitigação:**
- Marketing orgânico funciona (LinkedIn, grupos, boca-a-boca)
- R$ 500 de ads é suficiente para validação inicial

---

## ✅ Checklist Diário (Template)

```
DIA ____ / 30

MANHÃ (3h):
[ ] Tarefa prioritária do roadmap
[ ] Review código / arquitetura
[ ] Responder comunidade (Discord/GitHub)

TARDE (3h):
[ ] Desenvolvimento core
[ ] Testes / debugging
[ ] Documentação

NOITE (1h):
[ ] Marketing / conteúdo
[ ] Networking (LinkedIn, grupos)
[ ] Planejamento dia seguinte

BLOCKERS:
- 

CONQUISTAS:
- 

PRÓXIMO DIA:
- 
```

---

## 🎯 Milestone: Dia 30

**O que você terá:**

1. ✅ Marca validada (jurídico + INPI protocolado)
2. ✅ zaplivre.app online com landing page
3. ✅ 50-100+ emails de beta testers interessados
4. ✅ GitHub público com código funcionando
5. ✅ CLI enviando mensagens P2P criptografadas
6. ✅ Bootstrap node em produção (VPS)
7. ✅ Comunidade inicial (Discord/Slack)
8. ✅ Primeiros 20-50 stars no GitHub
9. ✅ Clareza sobre próximos 60 dias (Android alpha)

**O que você NÃO terá (ainda):**
- ❌ App móvel funcional
- ❌ 1000+ usuários
- ❌ Receita
- ❌ Equipe

**Mas isso é esperado!** Dia 30 é validação, não produto completo.

---

## 🚀 Próximos Passos IMEDIATOS (Esta Semana)

### Segunda-feira (Amanhã):
1. **Manhã:** Buscar advogado PI (3 orçamentos)
2. **Tarde:** Registrar redes sociais (@zaplivre)
3. **Noite:** Setup GitHub zaplivre/zaplivre

### Terça-feira:
1. **Manhã:** Consulta jurídica (se agendou)
2. **Tarde:** Renomear omega-core → zaplivre-core
3. **Noite:** Escrever README.md do projeto

### Quarta-feira:
1. **Manhã:** Desenvolver landing page (design)
2. **Tarde:** Desenvolver landing page (código)
3. **Noite:** Deploy Vercel + DNS zaplivre.app

### Quinta-feira:
1. **Manhã:** Implementar formulário beta (Typeform)
2. **Tarde:** Ligar para 10 distribuidores (pesquisa)
3. **Noite:** Blog post: "Por que criei ZapLivre"

### Sexta-feira:
1. **Manhã:** Provisionar VPS (bootstrap node)
2. **Tarde:** Deploy zaplivre-discovery
3. **Noite:** Testar CLI conectando ao bootstrap

---

## 💡 Dica Final

**Foco no MVP:**
- ✅ Landing page captando emails
- ✅ CLI funcionando (prova de conceito)
- ✅ Comunidade engajada

**Não fazer ainda:**
- ❌ App bonito e polido
- ❌ Todas as features
- ❌ Escalar infraestrutura

**Mantra:** "Feito é melhor que perfeito"

30 dias de hoje = landing page online + código funcionando + primeiros beta testers.

90 dias de hoje = app Android nas mãos de 10 distribuidores testando.

6 meses de hoje = 1.000+ usuários ativos.

**Vamos começar?** 🚀

---

**Quer que eu:**
1. Crie o HTML da landing page pronto pra usar?
2. Escreva o README.md do GitHub?
3. Faça o copy dos primeiros posts LinkedIn?
4. Desenhe os mockups do app Android?

Escolhe e eu faço agora! 💪
