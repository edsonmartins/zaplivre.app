import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter } from 'react-router-dom'
import { describe, expect, it, vi } from 'vitest'
import IncomingCallModal from '../IncomingCallModal'
import { setupTauri } from '../../test/tauriMock'

function renderModal(onClose = vi.fn()) {
  render(
    <MemoryRouter>
      <IncomingCallModal
        callId="call-123"
        callerPeerId="12D3KooWCallerPeer"
        onClose={onClose}
      />
    </MemoryRouter>
  )
  return onClose
}

describe('IncomingCallModal', () => {
  it('aceitar chama accept_call com o call_id certo e fecha', async () => {
    const { callsOf } = setupTauri({ accept_call: () => null })
    const user = userEvent.setup()
    const onClose = renderModal()

    await user.click(screen.getByRole('button', { name: /aceitar|accept|atender/i }))

    await waitFor(() => {
      const calls = callsOf('accept_call')
      expect(calls).toHaveLength(1)
      expect(calls[0].args).toMatchObject({ callId: 'call-123' })
      expect(onClose).toHaveBeenCalled()
    })
  })

  it('recusar chama reject_call e fecha', async () => {
    const { callsOf } = setupTauri({ reject_call: () => null })
    const user = userEvent.setup()
    const onClose = renderModal()

    await user.click(screen.getByRole('button', { name: /recusar|reject|rejeitar/i }))

    await waitFor(() => {
      expect(callsOf('reject_call')).toHaveLength(1)
      expect(onClose).toHaveBeenCalled()
    })
  })
})
