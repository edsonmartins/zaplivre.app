/**
 * Testes da tela de chamada de voz (CallView): cada botão precisa invocar o
 * comando Tauri certo com o callId da rota, o hangup precisa navegar de
 * volta (mesmo quando o core falha) e o timer precisa contar em mm:ss.
 */
import { act, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes } from 'react-router-dom'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import CallView from '../CallView'
import { VoipStateProvider } from '../../state/voipState'
import { setupTauri } from '../../test/tauriMock'

function renderCallView() {
  return render(
    <MemoryRouter initialEntries={['/call/call-1/12D3KooWRemotePeer']}>
      <VoipStateProvider>
        <Routes>
          <Route path="/call/:callId/:remotePeerId" element={<CallView localPeerId="PEER_A" />} />
          <Route path="/conversations" element={<div>rota-conversas</div>} />
          <Route path="/video-call/:callId/:remotePeerId" element={<div>rota-video</div>} />
        </Routes>
      </VoipStateProvider>
    </MemoryRouter>
  )
}

describe('CallView', () => {
  beforeEach(() => {
    localStorage.clear()
  })

  afterEach(() => {
    vi.useRealTimers()
    localStorage.clear()
  })

  it('botão Mute invoca toggle_mute com o callId da rota', async () => {
    const { callsOf } = setupTauri({ toggle_mute: () => null })
    const user = userEvent.setup()
    renderCallView()

    await user.click(screen.getByTitle('Mute'))

    await waitFor(() => {
      expect(callsOf('toggle_mute')).toHaveLength(1)
      expect(callsOf('toggle_mute')[0].args).toMatchObject({ callId: 'call-1' })
    })
  })

  it('botão Speaker invoca toggle_speakerphone com o callId', async () => {
    const { callsOf } = setupTauri({ toggle_speakerphone: () => null })
    const user = userEvent.setup()
    renderCallView()

    await user.click(screen.getByTitle('Speaker On'))

    await waitFor(() => {
      expect(callsOf('toggle_speakerphone')).toHaveLength(1)
      expect(callsOf('toggle_speakerphone')[0].args).toMatchObject({ callId: 'call-1' })
    })
  })

  it('botão Switch Camera invoca switch_camera com o callId', async () => {
    const { callsOf } = setupTauri({ switch_camera: () => null })
    const user = userEvent.setup()
    renderCallView()

    await user.click(screen.getByTitle('Switch Camera'))

    await waitFor(() => {
      expect(callsOf('switch_camera')).toHaveLength(1)
      expect(callsOf('switch_camera')[0].args).toMatchObject({ callId: 'call-1' })
    })
  })

  it('hangup invoca hangup_call e navega para as conversas', async () => {
    const { callsOf } = setupTauri({ hangup_call: () => null })
    const user = userEvent.setup()
    renderCallView()

    await user.click(screen.getByTitle('Hangup'))

    await waitFor(() => {
      expect(callsOf('hangup_call')).toHaveLength(1)
      expect(callsOf('hangup_call')[0].args).toMatchObject({ callId: 'call-1' })
      expect(screen.getByText('rota-conversas')).toBeInTheDocument()
    })
  })

  it('hangup rejeitando navega mesmo assim', async () => {
    setupTauri({
      hangup_call: () => {
        throw new Error('core indisponível')
      },
    })
    vi.spyOn(console, 'error').mockImplementation(() => {})
    const user = userEvent.setup()
    renderCallView()

    await user.click(screen.getByTitle('Hangup'))

    await waitFor(() => {
      expect(screen.getByText('rota-conversas')).toBeInTheDocument()
    })
  })

  it('timer da chamada mostra 01:05 após 65 segundos', async () => {
    setupTauri()
    vi.useFakeTimers()
    renderCallView()

    expect(screen.getByText('00:00')).toBeInTheDocument()

    act(() => {
      vi.advanceTimersByTime(65_000)
    })

    expect(screen.getByText('01:05')).toBeInTheDocument()
  })

  it('reflete estado mutado hidratado do voipState (localStorage)', () => {
    setupTauri()
    localStorage.setItem('mepassa:voip_state', JSON.stringify({ 'call-1': { isMuted: true } }))

    renderCallView()

    expect(screen.getByText(/Muted/)).toBeInTheDocument()
    // Com o mute ativo, o title do botão vira "Unmute"
    expect(screen.getByTitle('Unmute')).toBeInTheDocument()
  })

  it('botão de vídeo navega para a rota de video call', async () => {
    setupTauri()
    const user = userEvent.setup()
    renderCallView()

    await user.click(screen.getByTitle('Video call'))

    expect(await screen.findByText('rota-video')).toBeInTheDocument()
  })
})
