import React, { FC, ReactElement } from 'react';
import Chip from '@mui/material/Chip';
import { FlowType } from '../types';

type DataChipProps = {
  flowType: FlowType;
  small?: boolean;
  short?: boolean;
};

const DataChip = ({ flowType, small = true, short = false }: DataChipProps): ReactElement => {
  const label = () => {
    if (flowType === FlowType.Source) {
      return short ? 'S' : 'Source';
    }

    if (flowType === FlowType.Destination) {
      return short ? 'D' : 'Destination';
    }
  };
  const flowTypeColor = flowType === FlowType.Source ? 'forest.main' : 'forest.dark';
  const contrastFlowColor = flowType === FlowType.Source ? 'forest.dark' : 'forest.main';
  return (
    <Chip
      label={label()}
      size="small"
      sx={{
        '.MuiChip-label': { px: short ? '4px' : '8px' },
        fontSize: small ? '0.5125rem' : '0.7125rem',
        backgroundColor: flowTypeColor,
        color: contrastFlowColor,
        px: short ? '6px' : 0,
        border: `1px solid ${contrastFlowColor}`,
      }}
    />
  );
};
export default DataChip;
