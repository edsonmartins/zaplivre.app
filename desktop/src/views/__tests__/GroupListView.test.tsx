import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes } from 'react-router-dom'
import { describe, expect, it } from 'vitest'
import GroupListView from '../GroupListView'
import { setupTauri } from '../../test/tauriMock'
import { formatDate } from '../../utils/format'

/** Fixture de grupo no formato do comando get_groups */
function groupFixture(overrides: Record<string, unknown> = {}) {
  return {
    id: 'g1',
    name: 'Grupo Teste',
    description: null,
    member_count: 2,
    is_admin: true,
    created_at: 1_700_000_000, // SEGUNDOS (unixepoch do SQLite)
    ...overrides,
  }
}

function renderView(localPeerId: string | null = 'PEER_A') {
  return render(
    <MemoryRouter initialEntries={['/groups']}>
      <Routes>
        <Route path="/groups" element={<GroupListView localPeerId={localPeerId} />} />
        {/* Rota sentinela para verificar a navegação até o grupo */}
        <Route path="/group/:groupId" element={<div>rota-do-grupo</div>} />
      </Routes>
    </MemoryRouter>
  )
}

describe('GroupListView', () => {
  it('chama get_groups no mount e renderiza nome e data de criação (formatDate)', async () => {
    const { callsOf } = setupTauri({
      get_groups: () => [
        groupFixture({ name: 'Amigos da Facul' }),
        groupFixture({ id: 'g2', name: 'Família', is_admin: false }),
      ],
    })

    renderView()

    await waitFor(() => {
      expect(screen.getByText('Amigos da Facul')).toBeInTheDocument()
      expect(screen.getByText('Família')).toBeInTheDocument()
    })

    expect(callsOf('get_groups')).toHaveLength(1)

    // created_at em SEGUNDOS - interpretar como ms daria data de 1970
    const expectedDate = formatDate(1_700_000_000)
    expect(screen.getAllByText(expectedDate).length).toBeGreaterThan(0)
    expect(document.body.textContent ?? '').not.toMatch(/1970/)
  })

  it('mostra o empty state quando não há grupos', async () => {
    setupTauri({ get_groups: () => [] })

    renderView()

    expect(await screen.findByText(/no groups yet/i)).toBeInTheDocument()
    expect(
      screen.getByRole('button', { name: /create your first group/i })
    ).toBeInTheDocument()
  })

  it('cria grupo pelo diálogo e envia name/description trimados para create_group', async () => {
    const { callsOf } = setupTauri({
      get_groups: () => [],
      create_group: () => groupFixture({ id: 'g-novo', name: 'Amigos' }),
    })
    const user = userEvent.setup()

    renderView()

    const newGroupButton = await screen.findByRole('button', { name: /\+ new group/i })
    await user.click(newGroupButton)

    await user.type(
      await screen.findByPlaceholderText(/college friends/i),
      'Amigos'
    )
    await user.type(
      screen.getByPlaceholderText(/study group/i),
      'Grupo de estudos'
    )
    await user.click(screen.getByRole('button', { name: /^create$/i }))

    await waitFor(() => {
      const creates = callsOf('create_group')
      expect(creates).toHaveLength(1)
      expect(creates[0].args).toMatchObject({
        name: 'Amigos',
        description: 'Grupo de estudos',
      })
    })

    // Após criar, navega direto para a rota do grupo novo
    expect(await screen.findByText('rota-do-grupo')).toBeInTheDocument()
  })

  it('navega para a rota do grupo ao clicar num grupo da lista', async () => {
    setupTauri({
      get_groups: () => [groupFixture({ name: 'Grupo Clicável' })],
    })
    const user = userEvent.setup()

    renderView()

    await user.click(await screen.findByText('Grupo Clicável'))

    expect(await screen.findByText('rota-do-grupo')).toBeInTheDocument()
  })

  it('mostra estado de erro (sem crashar) quando get_groups rejeita', async () => {
    setupTauri({
      get_groups: () => {
        throw new Error('core indisponível')
      },
    })

    renderView()

    expect(await screen.findByText(/error loading groups/i)).toBeInTheDocument()
    expect(screen.getByText(/core indisponível/)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /try again/i })).toBeInTheDocument()
  })

  it('exibe o localPeerId no header quando informado', async () => {
    setupTauri({ get_groups: () => [] })

    renderView('PEER_LOCAL_XYZ')

    expect(await screen.findByText('PEER_LOCAL_XYZ')).toBeInTheDocument()
    expect(screen.getByRole('heading', { level: 1, name: 'Groups' })).toBeInTheDocument()
  })
})
