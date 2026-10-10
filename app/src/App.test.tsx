import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { App } from './App';

describe('App', () => {
  it('shows the product name on the start screen', () => {
    render(<App />);
    expect(screen.getByRole('heading', { level: 1, name: 'Lupa' })).toBeInTheDocument();
  });
});
