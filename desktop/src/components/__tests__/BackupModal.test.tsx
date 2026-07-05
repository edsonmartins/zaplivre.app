/**
 * DSK-09: o BackupModal exporta o backup Base64 da identidade via
 * export_identity_backup e permite copiá-lo. Sem esse fluxo, perder a
 * máquina = perder o peer ID.
 */
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import BackupModal from '../BackupModal'
import { setupTauri } from '../../test/tauriMock'

const BACKUP_PAYLOAD = 'QkFDS1VQLUJBU0U2NC1URVNURQ=='

/** jsdom não tem navigator.clipboard - instalar um mock observável */
function mockClipboard() {
  const writeText = vi.fn().mockResolvedValue(undefined)
  Object.defineProperty(window.navigator, 'clipboard', {
    value: { writeText },
    configurable: true,
  })
  return writeText
}

describe('BackupModal', () => {
  it('no mount chama export_identity_backup e exibe o payload', async () => {
    const { callsOf } = setupTauri({
      export_identity_backup: () => BACKUP_PAYLOAD,
    })

    render(<BackupModal onClose={vi.fn()} />)

    // O backup retornado pelo core aparece no textarea (readOnly)
    expect(await screen.findByDisplayValue(BACKUP_PAYLOAD)).toBeInTheDocument()
    expect(callsOf('export_identity_backup')).toHaveLength(1)
  })

  it('copiar backup escreve o payload no clipboard e mostra "Copiado"', async () => {
    setupTauri({ export_identity_backup: () => BACKUP_PAYLOAD })
    const user = userEvent.setup()
    const writeText = mockClipboard()

    render(<BackupModal onClose={vi.fn()} />)
    await screen.findByDisplayValue(BACKUP_PAYLOAD)

    await user.click(screen.getByRole('button', { name: /copiar backup/i }))

    await waitFor(() => {
      expect(writeText).toHaveBeenCalledWith(BACKUP_PAYLOAD)
      expect(screen.getByRole('button', { name: /copiado/i })).toBeInTheDocument()
    })
  })

  it('erro do invoke vira mensagem de erro (sem textarea)', async () => {
    setupTauri({
      export_identity_backup: () => {
        throw new Error('keychain bloqueado')
      },
    })

    render(<BackupModal onClose={vi.fn()} />)

    expect(await screen.findByText(/keychain bloqueado/i)).toBeInTheDocument()
    // No modo de erro o textarea com o backup não é renderizado
    expect(screen.queryByRole('textbox')).not.toBeInTheDocument()
  })

  it('botão de fechar chama onClose', async () => {
    setupTauri({ export_identity_backup: () => BACKUP_PAYLOAD })
    const user = userEvent.setup()
    const onClose = vi.fn()

    render(<BackupModal onClose={onClose} />)
    await screen.findByDisplayValue(BACKUP_PAYLOAD)

    await user.click(screen.getByTitle('Fechar'))

    expect(onClose).toHaveBeenCalledTimes(1)
  })
})
