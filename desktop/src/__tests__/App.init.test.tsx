/**
 * Boot do App: init_client -> listen_on (TCP + QUIC) -> bootstrap.
 *
 * Regras que este spec protege:
 * - listen_on e bootstrap têm try/catch INDEPENDENTES - a falha de um
 *   não pode derrubar a inicialização
 * - init_client falhando mostra a tela "Initialization Failed"
 * - a rota inicial é decidida pelo resultado do init (conversas vs onboarding)
 * - voip:call_ended só fecha o modal da chamada certa
 */
import { render, screen, waitFor } from '@testing-library/react'
import { emit } from '@tauri-apps/api/event'
import { MemoryRouter } from 'react-router-dom'
import { describe, expect, it } from 'vitest'
import AppWithProviders from '../App'
import { setupTauri } from '../test/tauriMock'

type Handlers = Parameters<typeof setupTauri>[0]

function setupAppMocks(overrides: Handlers = {}) {
  return setupTauri({
    init_client: () => '12D3KooWLocalPeer',
    listen_on: () => null,
    bootstrap: () => null,
    list_conversations: () => [],
    register_video_frame_callback: () => null,
    ...overrides,
  })
}

function renderApp(initialEntries: string[] = ['/conversations']) {
  render(
    <MemoryRouter initialEntries={initialEntries}>
      <AppWithProviders />
    </MemoryRouter>
  )
}

async function waitForBoot() {
  await waitFor(() => {
    expect(screen.queryByText(/carregando zaplivre/i)).not.toBeInTheDocument()
  })
}

describe('App - inicialização', () => {
  it('init feliz: sai do loading, escuta em TCP+QUIC e faz bootstrap do DHT', async () => {
    const { callsOf } = setupAppMocks()

    renderApp()
    await waitForBoot()

    expect(callsOf('init_client')).toHaveLength(1)

    // Exatamente 2 listen_on, na ordem TCP -> QUIC, com os multiaddrs exatos
    const listens = callsOf('listen_on')
    expect(listens).toHaveLength(2)
    expect(listens[0].args).toMatchObject({ multiaddr: '/ip4/0.0.0.0/tcp/0' })
    expect(listens[1].args).toMatchObject({
      multiaddr: '/ip4/0.0.0.0/udp/0/quic-v1',
    })

    expect(callsOf('bootstrap')).toHaveLength(1)
  })

  it('init_client rejeitando mostra a tela de erro com a mensagem', async () => {
    setupAppMocks({
      init_client: () => {
        throw new Error('keychain corrompido')
      },
    })

    renderApp()

    expect(await screen.findByText(/initialization failed/i)).toBeInTheDocument()
    expect(screen.getByText(/keychain corrompido/)).toBeInTheDocument()
  })

  it('listen_on rejeitando NÃO derruba o app (try/catch independente)', async () => {
    const { callsOf } = setupAppMocks({
      listen_on: () => {
        throw new Error('porta ocupada')
      },
    })

    renderApp()
    await waitForBoot()

    // Mesmo com os dois listen_on falhando, o boot segue até o bootstrap
    // e o app cai nas conversas, sem tela de erro
    expect(callsOf('bootstrap')).toHaveLength(1)
    expect(screen.queryByText(/initialization failed/i)).not.toBeInTheDocument()
    expect(await screen.findByRole('heading', { name: 'ZapLivre' })).toBeInTheDocument()
  })

  it('bootstrap rejeitando NÃO derruba o app', async () => {
    setupAppMocks({
      bootstrap: () => {
        throw new Error('sem rede')
      },
    })

    renderApp()
    await waitForBoot()

    expect(screen.queryByText(/initialization failed/i)).not.toBeInTheDocument()
    expect(await screen.findByRole('heading', { name: 'ZapLivre' })).toBeInTheDocument()
  })

  it('voip:call_ended de OUTRA chamada não fecha o modal da chamada recebida', async () => {
    setupAppMocks()

    renderApp()
    await waitForBoot()

    await emit('voip:incoming_call', {
      call_id: 'call-111',
      from_peer_id: '12D3KooWCallerPeer',
    })
    expect(await screen.findByRole('button', { name: /atender/i })).toBeInTheDocument()

    // Encerrou uma chamada diferente - o modal da call-111 tem que ficar
    await emit('voip:call_ended', { call_id: 'call-999', reason: 'CallerHungUp' })

    expect(screen.getByRole('button', { name: /atender/i })).toBeInTheDocument()
  })

  it('rota inicial "/": com init bem-sucedido navega para as conversas', async () => {
    const { callsOf } = setupAppMocks()

    renderApp(['/'])
    await waitForBoot()

    expect(await screen.findByRole('heading', { name: 'ZapLivre' })).toBeInTheDocument()
    await waitFor(() => {
      expect(callsOf('list_conversations').length).toBeGreaterThan(0)
    })
  })

  it('rota inicial "/": init falho sem mensagem de erro cai no onboarding', async () => {
    // O App decide a rota por isInitialized: falha COM mensagem vira a tela
    // "Initialization Failed"; falha sem mensagem (errorMessage vazio) é o
    // único caminho real para o onboarding via boot
    setupAppMocks({
      init_client: () => Promise.reject(''),
    })

    renderApp(['/'])
    await waitForBoot()

    expect(
      await screen.findByRole('heading', { name: /bem-vindo ao zaplivre/i })
    ).toBeInTheDocument()
  })
})
