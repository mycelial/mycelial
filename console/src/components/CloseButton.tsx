import React from 'react';
import IconButton from '@mui/material/IconButton';
import CloseIcon from '@mui/icons-material/Close';

type CloseButtonProps = {
  onClick: () => void;
  color?: string;
};

export default function CloseButton({ onClick, color = '#fc4445' }: CloseButtonProps) {
  return (
    <IconButton
      onClick={onClick}
      sx={{
        color,
        float: 'right',
        mb: 1,
        padding: 0,
      }}
      size="small"
      className="deleteNodeButton"
      role="button"
    >
      <CloseIcon sx={{ height: '.7em', width: '.7em' }} />
    </IconButton>
  );
}
