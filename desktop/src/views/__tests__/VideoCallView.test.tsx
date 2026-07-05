/**
 * Testes da tela de vídeo chamada (VideoCallView): o mount precisa registrar
 * o callback de frames e habilitar o vídeo h264 no core, o toggle precisa
 * alternar enable_video/disable_video, o hangup navega de volta e os eventos
 * de frame (voip:video_frame_rgba / voip:video_frame) não podem quebrar a UI.
 *
 * jsdom não implementa canvas 2d: mockamos getContext com um contexto fake
 * e stubamos ImageData (jsdom sem o pacote canvas não a expõe).
 */
import { act, cleanup, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { emit } from '@tauri-apps/api/event'
import { MemoryRouter, Route, Routes } from 'react-router-dom'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import VideoCallView from '../VideoCallView'
import { setupTauri } from '../../test/tauriMock'

const putImageData = vi.fn()
const drawImage = vi.fn()
const fakeCtx = { putImageData, drawImage } as unknown as CanvasRenderingContext2D

/** jsdom não define ImageData - stub mínimo compatível com o uso da view */
class FakeImageData {
  constructor(
    public data: Uint8ClampedArray,
    public width: number,
    public height: number
  ) {}
}

function renderVideoCallView() {
  return render(
    <MemoryRouter initialEntries={['/video-call/call-1/12D3KooWRemotePeer']}>
      <Routes>
        <Route path="/video-call/:callId/:remotePeerId" element={<VideoCallView />} />
        <Route path="/conversations" element={<div>rota-conversas</div>} />
      </Routes>
    </MemoryRouter>
  )
}

function setupVideoMocks(overrides: Record<string, () => unknown> = {}) {
  return setupTauri({
    register_video_frame_callback: () => null,
    enable_video: () => null,
    disable_video: () => null,
    ...overrides,
  })
}

/** Payload base64 de um frame RGBA width x height (4 bytes por pixel) */
function rgbaFrameB64(width: number, height: number) {
  const bytes = new Uint8Array(width * height * 4).fill(128)
  return btoa(String.fromCharCode(...bytes))
}

describe('VideoCallView', () => {
  beforeEach(() => {
    putImageData.mockClear()
    drawImage.mockClear()
    vi.stubGlobal('ImageData', FakeImageData)
    vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(fakeCtx)
  })

  afterEach(async () => {
    // Desmonta AGORA (antes do clearMocks do setup global) e dá um tick para
    // os unlisten assíncronos rodarem enquanto o mock de eventos ainda existe
    cleanup()
    await act(async () => {})
    vi.unstubAllGlobals()
    vi.restoreAllMocks()
  })

  it('no mount registra o callback de frames e habilita vídeo h264', async () => {
    const { callsOf } = setupVideoMocks()
    renderVideoCallView()

    await waitFor(() => {
      expect(callsOf('register_video_frame_callback')).toHaveLength(1)
      expect(callsOf('enable_video')).toHaveLength(1)
      expect(callsOf('enable_video')[0].args).toMatchObject({
        callId: 'call-1',
        codec: 'h264',
      })
    })
  })

  it('desligar o vídeo invoca disable_video com o callId', async () => {
    const { callsOf } = setupVideoMocks()
    const user = userEvent.setup()
    renderVideoCallView()

    await user.click(screen.getByRole('button', { name: 'Video On' }))

    await waitFor(() => {
      expect(callsOf('disable_video')).toHaveLength(1)
      expect(callsOf('disable_video')[0].args).toMatchObject({ callId: 'call-1' })
      expect(screen.getByRole('button', { name: 'Video Off' })).toBeInTheDocument()
    })
  })

  it('religar o vídeo dispara um segundo enable_video', async () => {
    const { callsOf } = setupVideoMocks()
    const user = userEvent.setup()
    renderVideoCallView()

    // enable_video do mount
    await waitFor(() => expect(callsOf('enable_video')).toHaveLength(1))

    await user.click(screen.getByRole('button', { name: 'Video On' }))
    await waitFor(() => expect(callsOf('disable_video')).toHaveLength(1))

    await user.click(screen.getByRole('button', { name: 'Video Off' }))

    await waitFor(() => {
      expect(callsOf('enable_video')).toHaveLength(2)
      expect(callsOf('enable_video')[1].args).toMatchObject({
        callId: 'call-1',
        codec: 'h264',
      })
    })
  })

  it('hangup invoca hangup_call e navega para as conversas', async () => {
    const { callsOf } = setupVideoMocks({ hangup_call: () => null })
    const user = userEvent.setup()
    renderVideoCallView()

    await user.click(screen.getByRole('button', { name: 'Hangup' }))

    await waitFor(() => {
      expect(callsOf('hangup_call')).toHaveLength(1)
      expect(callsOf('hangup_call')[0].args).toMatchObject({ callId: 'call-1' })
      expect(screen.getByText('rota-conversas')).toBeInTheDocument()
    })
  })

  it('evento voip:video_frame_rgba desenha o frame no canvas sem explodir', async () => {
    const { callsOf } = setupVideoMocks()
    renderVideoCallView()

    // Garante que os listeners do mount já foram registrados
    await waitFor(() => expect(callsOf('enable_video')).toHaveLength(1))

    await act(async () => {
      await emit('voip:video_frame_rgba', {
        call_id: 'call-1',
        width: 2,
        height: 2,
        size: 16,
        data_b64: rgbaFrameB64(2, 2),
      })
    })

    await waitFor(() => {
      expect(putImageData).toHaveBeenCalledTimes(1)
    })
  })

  it('evento voip:video_frame sem VideoDecoder mostra erro sem quebrar', async () => {
    const { callsOf } = setupVideoMocks()
    renderVideoCallView()

    await waitFor(() => expect(callsOf('enable_video')).toHaveLength(1))

    // jsdom não tem VideoDecoder - a UI precisa degradar com mensagem visível
    await act(async () => {
      await emit('voip:video_frame', {
        call_id: 'call-1',
        width: 2,
        height: 2,
        size: 4,
        data_b64: btoa(String.fromCharCode(0, 0, 0, 1, 0x65)),
      })
    })

    expect(
      await screen.findByText(/VideoDecoder nao esta disponivel/i)
    ).toBeInTheDocument()
  })

  it('enable_video rejeitando não quebra a tela', async () => {
    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
    const { callsOf } = setupVideoMocks({
      enable_video: () => {
        throw new Error('sem câmera')
      },
    })
    renderVideoCallView()

    await waitFor(() => expect(callsOf('enable_video')).toHaveLength(1))

    // A tela continua de pé, com os controles utilizáveis
    expect(screen.getByText('Remote video')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Video On' })).toBeInTheDocument()
    expect(errorSpy).toHaveBeenCalled()
  })

  it('unmount limpa os listeners: frame após unmount não desenha', async () => {
    const { callsOf } = setupVideoMocks()
    const { unmount } = renderVideoCallView()

    await waitFor(() => expect(callsOf('enable_video')).toHaveLength(1))

    unmount()
    // Os unlisten são resolvidos de forma assíncrona
    await act(async () => {})

    putImageData.mockClear()
    await emit('voip:video_frame_rgba', {
      call_id: 'call-1',
      width: 2,
      height: 2,
      size: 16,
      data_b64: rgbaFrameB64(2, 2),
    })

    expect(putImageData).not.toHaveBeenCalled()
  })
})
