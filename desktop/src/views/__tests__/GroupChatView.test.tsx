import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes } from 'react-router-dom'
import { afterEach, describe, expect, it, vi } from 'vitest'
import GroupChatView from '../GroupChatView'
import { setupTauri } from '../../test/tauriMock'
import { formatMessageTime } from '../../utils/format'

/**
 * Como o GroupChatView carrega dados (mapeado do componente):
 * - `get_local_peer_id` → preenche localPeerIdRef (decide is_own_message)
 * - `get_groups` → filtra pelo :groupId da rota para achar o grupo do header
 * - `get_group_messages { groupId }` → mensagens (campo content_plaintext),
 *   com auto-refresh a cada 3s
 * - `get_group_members { groupId }` → só quando o modal Group Info abre
 * - Add Member usa window.prompt e Leave Group usa window.confirm
 */

/** Fixture de grupo no formato do comando get_groups */
function groupFixture(overrides: Record<string, unknown> = {}) {
  return {
    id: 'g1',
    name: 'Grupo Teste',
    description: null,
    member_count: 2,
    is_admin: true,
    created_at: 1_700_000_000,
    ...overrides,
  }
}

/** Fixture de mensagem no formato bruto do comando get_group_messages */
function groupMessageFixture(overrides: Record<string, unknown> = {}) {
  return {
    message_id: 'gmsg-1',
    sender_peer_id: 'PEER_B',
    content_plaintext: 'Olá, grupo!',
    created_at: 1_700_000_000, // SEGUNDOS (unixepoch do SQLite)
    ...overrides,
  }
}

/** Mocks base dos comandos de carregamento */
function baseCommands(overrides: Record<string, (args?: Record<string, unknown>) => unknown> = {}) {
  return {
    get_local_peer_id: () => 'PEER_A',
    get_groups: () => [groupFixture()],
    get_group_messages: () => [groupMessageFixture()],
    ...overrides,
  }
}

function renderChat() {
  return render(
    <MemoryRouter initialEntries={['/group/g1']}>
      <Routes>
        <Route path="/group/:groupId" element={<GroupChatView />} />
        {/* Rota sentinela para verificar a navegação ao sair do grupo */}
        <Route path="/groups" element={<div>rota-lista-grupos</div>} />
      </Routes>
    </MemoryRouter>
  )
}

afterEach(() => {
  vi.restoreAllMocks()
})

describe('GroupChatView', () => {
  it('carrega grupo e mensagens no mount (get_groups + get_group_messages do :groupId)', async () => {
    const { callsOf } = setupTauri(baseCommands())

    renderChat()

    expect(await screen.findByText('Grupo Teste')).toBeInTheDocument()
    expect(await screen.findByText('Olá, grupo!')).toBeInTheDocument()

    expect(callsOf('get_groups').length).toBeGreaterThan(0)
    const loads = callsOf('get_group_messages')
    expect(loads.length).toBeGreaterThan(0)
    expect(loads[0].args).toMatchObject({ groupId: 'g1' })
  })

  it('envia mensagem via send_group_message com args certos e limpa o input', async () => {
    const { callsOf } = setupTauri(
      baseCommands({
        get_group_messages: () => [],
        send_group_message: () => null,
      })
    )
    const user = userEvent.setup()

    renderChat()

    const input = await screen.findByPlaceholderText(/type a message/i)
    await user.type(input, 'mensagem para o grupo')
    await user.keyboard('{Enter}')

    await waitFor(() => {
      const sends = callsOf('send_group_message')
      expect(sends).toHaveLength(1)
      expect(sends[0].args).toMatchObject({
        groupId: 'g1',
        content: 'mensagem para o grupo',
      })
    })

    expect(input).toHaveValue('')
  })

  it('não envia quando o input está vazio ou só tem espaços', async () => {
    const { callsOf } = setupTauri(
      baseCommands({
        get_group_messages: () => [],
        send_group_message: () => null,
      })
    )
    const user = userEvent.setup()

    renderChat()

    const input = await screen.findByPlaceholderText(/type a message/i)

    // Enter com input vazio
    await user.click(input)
    await user.keyboard('{Enter}')

    // Enter com input só de espaços
    await user.type(input, '   ')
    await user.keyboard('{Enter}')

    await waitFor(() => {
      expect(callsOf('send_group_message')).toHaveLength(0)
    })
  })

  it('adiciona membro via add_group_member com o peer id trimado (window.prompt)', async () => {
    const { callsOf } = setupTauri(
      baseCommands({
        get_group_members: () => ['PEER_A', 'PEER_B'],
        add_group_member: () => null,
      })
    )
    vi.spyOn(window, 'prompt').mockReturnValue('   PEER_NOVO   ')
    const user = userEvent.setup()

    renderChat()

    await user.click(await screen.findByRole('button', { name: /group info/i }))
    await user.click(await screen.findByRole('button', { name: /add member/i }))

    await waitFor(() => {
      const adds = callsOf('add_group_member')
      expect(adds).toHaveLength(1)
      expect(adds[0].args).toMatchObject({
        groupId: 'g1',
        peerId: 'PEER_NOVO', // sem os espaços digitados no prompt
      })
    })
  })

  it('sai do grupo via leave_group e navega para a lista de grupos', async () => {
    const { callsOf } = setupTauri(
      baseCommands({
        get_group_members: () => ['PEER_A'],
        leave_group: () => null,
      })
    )
    vi.spyOn(window, 'confirm').mockReturnValue(true)
    const user = userEvent.setup()

    renderChat()

    await user.click(await screen.findByRole('button', { name: /group info/i }))
    await user.click(await screen.findByRole('button', { name: /leave group/i }))

    await waitFor(() => {
      const leaves = callsOf('leave_group')
      expect(leaves).toHaveLength(1)
      expect(leaves[0].args).toMatchObject({ groupId: 'g1' })
    })

    expect(await screen.findByText('rota-lista-grupos')).toBeInTheDocument()
  })

  it('renderiza o horário das mensagens com formatMessageTime (created_at em SEGUNDOS)', async () => {
    setupTauri(
      baseCommands({
        get_group_messages: () => [
          groupMessageFixture({ content_plaintext: 'Mensagem com hora' }),
        ],
      })
    )

    renderChat()

    expect(await screen.findByText('Mensagem com hora')).toBeInTheDocument()

    const expectedTime = formatMessageTime(1_700_000_000)
    expect(screen.getByText(expectedTime)).toBeInTheDocument()
    // Regressão: interpretar segundos como ms daria data de 1970
    expect(document.body.textContent ?? '').not.toMatch(/1970/)
  })

  it('mostra erro (sem crashar) quando send_group_message rejeita', async () => {
    setupTauri(
      baseCommands({
        get_group_messages: () => [],
        send_group_message: () => {
          throw new Error('rede indisponível')
        },
      })
    )
    const user = userEvent.setup()

    renderChat()

    const input = await screen.findByPlaceholderText(/type a message/i)
    await user.type(input, 'vai falhar')
    await user.keyboard('{Enter}')

    expect(await screen.findByText(/failed to send message/i)).toBeInTheDocument()
    expect(screen.getByText(/rede indisponível/)).toBeInTheDocument()
    // Input continua utilizável após o erro
    expect(input).toBeEnabled()
  })

  it('mostra feedback de erro quando add_group_member rejeita', async () => {
    setupTauri(
      baseCommands({
        get_group_members: () => ['PEER_A'],
        add_group_member: () => {
          throw new Error('peer não encontrado')
        },
      })
    )
    vi.spyOn(window, 'prompt').mockReturnValue('PEER_INEXISTENTE')
    const user = userEvent.setup()

    renderChat()

    await user.click(await screen.findByRole('button', { name: /group info/i }))
    await user.click(await screen.findByRole('button', { name: /add member/i }))

    expect(await screen.findByText(/failed to add member/i)).toBeInTheDocument()
    expect(screen.getByText(/peer não encontrado/)).toBeInTheDocument()
  })
})
