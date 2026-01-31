# Self-Hosting (VPS) - Bootstrap/DHT

Este guia publica os servidores de Bootstrap/DHT do MePassa em uma VPS usando Docker + Swarm + Traefik.

## ✅ Pré-requisitos

- VPS com Docker instalado
- DNS apontando para a VPS
- Traefik já rodando em Swarm (network externa `traefik-network`)
- Portas liberadas no firewall

## 1) DNS

Crie registros A:

- `dht1.associahub.com.br` → IP da VPS
- `dht2.associahub.com.br` → IP da VPS

## 2) Build da imagem

Na VPS, dentro do repositório:

```bash
docker build -f server/bootstrap/Dockerfile -t mepassa-bootstrap:latest .
```

## 3) Configurar envs

```bash
sudo mkdir -p /etc/mepassa
sudo cp server/bootstrap/.env.example /etc/mepassa/bootstrap.env
sudo cp server/bootstrap/.env.bootstrap-2.example /etc/mepassa/bootstrap-2.env
sudo nano /etc/mepassa/bootstrap.env
sudo nano /etc/mepassa/bootstrap-2.env
```

Edite:
- `bootstrap.env` → `PEER_ID_SEED=bootstrap-1`
- `bootstrap-2.env` → `PEER_ID_SEED=bootstrap-2`

## 4) Deploy com Docker Stack

```bash
docker stack deploy -c server/bootstrap/stack.yml mepassa
```

## 5) Verificação

```bash
docker service ls

docker service logs -f mepassa_bootstrap-node

docker service logs -f mepassa_bootstrap-node-2
```

Health checks:

```bash
curl https://dht1.associahub.com.br/health
curl https://dht2.associahub.com.br/health
```

## 6) Firewall

Portas obrigatórias para P2P (libp2p TCP):

```bash
sudo ufw allow 4001/tcp
sudo ufw allow 4002/tcp
```

As portas 8000/8001 podem ficar **fechadas** se o health estiver atrás do Traefik.

## 7) Peer IDs e multiaddrs

Nos logs de cada serviço, pegue o Peer ID:

```
Peer ID: 12D3KooW...
```

Multiaddrs para os clientes:
- Node 1: `/dns4/dht1.associahub.com.br/tcp/4001`
- Node 2: `/dns4/dht2.associahub.com.br/tcp/4002`

## 8) Atualizar clientes

Edite `core/src/ffi/client.rs` para usar seus bootstraps públicos:

```rust
let custom_bootstrap_peers = vec![
    ("/dns4/dht1.associahub.com.br/tcp/4001", "12D3KooW..."),
    ("/dns4/dht2.associahub.com.br/tcp/4002", "12D3KooW..."),
];
```

## 9) Rotação/Atualização

Para atualizar a imagem:

```bash
docker build -f server/bootstrap/Dockerfile -t mepassa-bootstrap:latest .
docker stack deploy -c server/bootstrap/stack.yml mepassa
```

## Troubleshooting

- Sem peers conectando: verifique portas 4001/4002 liberadas e DNS correto.
- Health 404: confirme se o Traefik está na network `traefik-network`.
- Certificado não emite: confira DNS e logs do Traefik.
