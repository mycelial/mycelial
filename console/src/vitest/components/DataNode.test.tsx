import { expect, test } from 'vitest';
import DataNode from '../../components/DataNode';
import { cleanup, render, screen } from '@testing-library/react';
import React from 'react';

const testProps = {
  id: 'test',
  data: {
    id: 'test',
    display_name: 'test display name',
    label: 'test',
    type: 'test',
    source: true,
    destination: true,
    displayLabel: 'test display label',
  },
};

const wrapNodeProps = {
  selected: false,
  type: '',
  zIndex: 0,
  isConnectable: false,
  xPos: 0,
  yPos: 0,
  dragging: false,
};

test.skip('DataNode renders', () => {
  vi.mock('reactflow');
  render(<DataNode {...wrapNodeProps} {...testProps} />);
  const divElement = screen.getByRole('button');
  expect(divElement).not.toBeNull();

  expect(divElement).toHaveTextContent('X');
  expect(screen.getByRole('contentinfo')).toHaveTextContent('test display name');
});
