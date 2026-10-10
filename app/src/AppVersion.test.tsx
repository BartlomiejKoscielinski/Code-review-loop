import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { act, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { AppVersion } from './AppVersion';

afterEach(() => {
  clearMocks();
});

/** Renders AppVersion with a mocked core and waits until the call settles. */
async function renderWithCore(respond: () => unknown) {
  let calls = 0;
  mockIPC(() => {
    calls += 1;
    return respond();
  });
  const view = render(<AppVersion />);
  await waitFor(() => expect(calls).toBe(1));
  await act(async () => {});
  return view;
}

describe('AppVersion', () => {
  it('shows the name and version reported by the core', async () => {
    await renderWithCore(() => ({ name: 'Lupa', version: '0.1.0' }));
    expect(screen.getByText('Lupa 0.1.0')).toBeInTheDocument();
  });

  it('renders nothing when the core reports an error', async () => {
    const { container } = await renderWithCore(() => {
      throw { code: 'internal', message: 'failed' };
    });
    expect(container).toBeEmptyDOMElement();
  });

  it('renders nothing when the response does not match the contract', async () => {
    const { container } = await renderWithCore(() => ({ name: 'Lupa' }));
    expect(container).toBeEmptyDOMElement();
  });
});
