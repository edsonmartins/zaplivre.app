import { render, screen } from '@testing-library/react'
import { MemoryRouter } from 'react-router-dom'
import { describe, expect, it } from 'vitest'
import OnboardingView from '../OnboardingView'

describe('OnboardingView', () => {
  it('desabilita "Get Started" enquanto o client não inicializou', () => {
    render(
      <MemoryRouter>
        <OnboardingView localPeerId={null} />
      </MemoryRouter>
    )

    const button = screen.getByRole('button', { name: /initializing/i })
    expect(button).toBeDisabled()
  })

  it('habilita "Get Started" e mostra o peer ID quando inicializado', () => {
    render(
      <MemoryRouter>
        <OnboardingView localPeerId="12D3KooWTestPeer" />
      </MemoryRouter>
    )

    expect(screen.getByText('12D3KooWTestPeer')).toBeInTheDocument()
    const button = screen.getByRole('button', { name: /get started/i })
    expect(button).toBeEnabled()
  })
})
