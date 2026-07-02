import '@testing-library/jest-dom/vitest'
import { cleanup } from '@testing-library/react'
import { clearMocks } from '@tauri-apps/api/mocks'
import { afterEach, vi } from 'vitest'

// jsdom não implementa scrollIntoView (usado nos chats)
Element.prototype.scrollIntoView = vi.fn()

afterEach(() => {
  cleanup()
  clearMocks()
})
