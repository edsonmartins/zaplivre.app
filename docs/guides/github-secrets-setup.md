# GitHub Secrets Setup (ZapLivre)

Este guia explica como configurar os secrets usados nos workflows do GitHub Actions.

## Secrets necessários

- `MESSAGE_STORE_URL`
- `PUSH_SERVER_URL`

## Passo a passo (GitHub UI)

1) Acesse o repositório no GitHub
2) Clique em **Settings**
3) No menu lateral, vá em **Secrets and variables → Actions**
4) Clique em **New repository secret**
5) Adicione:

- **Name:** `MESSAGE_STORE_URL`
  **Value:** `https://store.associahub.com.br`

- **Name:** `PUSH_SERVER_URL`
  **Value:** `https://push.associahub.com.br`

## Referência nos workflows

Os workflows já estão configurados para ler:

```
${{ secrets.MESSAGE_STORE_URL }}
${{ secrets.PUSH_SERVER_URL }}
```

## Verificação rápida

Dispare um workflow (push/PR) e verifique se não há erros de env ausente.

