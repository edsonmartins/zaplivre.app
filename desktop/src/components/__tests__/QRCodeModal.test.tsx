/**
 * QRCodeModal: busca os endereços de escuta (get_listening_addresses),
 * monta o QR "peerId@multiaddr" (formato que o iOS parseia) e permite
 * copiar o peer ID.
 */
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import QRCodeModal from '../QRCodeModal'
import { setupTauri } from '../../test/tauriMock'

const PEER_ID = '12D3KooWLocalPeer'
const LAN_ADDR = '/ip4/192.168.0.10/tcp/4001'

/** jsdom não tem navigator.clipboard - instalar um mock observável */
function mockClipboard() {
  const writeText = vi.fn().mockResolvedValue(undefined)
  Object.defineProperty(window.navigator, 'clipboard', {
    value: { writeText },
    configurable: true,
  })
  return writeText
}

/** O QR é um SVG 220x220 do qrcode.react (os ícones lucide são svgs menores) */
function queryQrSvg() {
  return document.querySelector('svg[height="220"][width="220"]')
}

describe('QRCodeModal', () => {
  it('no mount chama get_listening_addresses e renderiza o QR com o peer ID', async () => {
    const { callsOf } = setupTauri({
      get_listening_addresses: () => [LAN_ADDR],
    })

    render(<QRCodeModal localPeerId={PEER_ID} onClose={vi.fn()} />)

    // Enquanto carrega não há QR; depois do invoke o SVG aparece
    await waitFor(() => {
      expect(queryQrSvg()).toBeInTheDocument()
    })
    expect(callsOf('get_listening_addresses')).toHaveLength(1)
    // O peer ID (parte do payload do QR) é exibido no modal
    expect(screen.getByText(PEER_ID)).toBeInTheDocument()
    // O endereço roteável escolhido para o QR também é exibido
    expect(screen.getByText(LAN_ADDR)).toBeInTheDocument()
  })

  it('sem endereço roteável mostra o aviso (QR só com o peer ID)', async () => {
    setupTauri({ get_listening_addresses: () => [] })

    render(<QRCodeModal localPeerId={PEER_ID} onClose={vi.fn()} />)

    expect(
      await screen.findByText(/nenhum endereço roteável encontrado/i)
    ).toBeInTheDocument()
    expect(queryQrSvg()).toBeInTheDocument()
    expect(screen.getByText(PEER_ID)).toBeInTheDocument()
  })

  it('"Atualizar endereço" chama get_listening_addresses de novo', async () => {
    const { callsOf } = setupTauri({
      get_listening_addresses: () => [LAN_ADDR],
    })
    const user = userEvent.setup()

    render(<QRCodeModal localPeerId={PEER_ID} onClose={vi.fn()} />)
    await waitFor(() => expect(queryQrSvg()).toBeInTheDocument())

    await user.click(screen.getByRole('button', { name: /atualizar endereço/i }))

    await waitFor(() => {
      expect(callsOf('get_listening_addresses')).toHaveLength(2)
    })
  })

  it('copiar escreve o peer ID no clipboard e confirma visualmente', async () => {
    setupTauri({ get_listening_addresses: () => [LAN_ADDR] })
    const user = userEvent.setup()
    const writeText = mockClipboard()

    render(<QRCodeModal localPeerId={PEER_ID} onClose={vi.fn()} />)
    await waitFor(() => expect(queryQrSvg()).toBeInTheDocument())

    // Há dois botões de copiar (ícone ao lado do peer ID e o botão grande)
    await user.click(screen.getAllByRole('button', { name: /copiar peer id/i })[0])

    await waitFor(() => {
      expect(writeText).toHaveBeenCalledWith(PEER_ID)
      expect(screen.getByText(/peer id copiado/i)).toBeInTheDocument()
    })
  })

  it('botão de fechar chama onClose', async () => {
    setupTauri({ get_listening_addresses: () => [LAN_ADDR] })
    const user = userEvent.setup()
    const onClose = vi.fn()

    render(<QRCodeModal localPeerId={PEER_ID} onClose={onClose} />)
    await waitFor(() => expect(queryQrSvg()).toBeInTheDocument())

    await user.click(screen.getByRole('button', { name: /fechar/i }))

    expect(onClose).toHaveBeenCalledTimes(1)
  })
})
