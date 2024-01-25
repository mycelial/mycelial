import { createTheme } from '@mui/material/styles';
import AeonikMono from './fonts/AeonikMono.ttf';
import AeonikFono from './fonts/AeonikFono.ttf';
// import { PaletteOptions } from '@mui/material/styles';
// declare module '@mui/material/styles' {
//   interface MycelialPalette extends PaletteOptions {
//     forest: PaletteOptions;
//     toadstool?: PaletteOptions;
//     brightGreen: PaletteOptions;
//   }
// }

const theme = createTheme({
  palette: {
    primary: { main: '#1a237e', contrastText: '#fff', light: '#586dae' },
    secondary: {
      main: '#1b5e20',
    },
    forest: {
      main: '#a5d6a7',
      light: '#bfd1bf',
      dark: '#3a554c',
    },
    brightGreen: { main: '#92fc95' },
    toadstool: { main: '#fc1717' },
  },
  spacing: 4,
  typography: {
    fontSize: 12,
    fontFamily: AeonikFono,
  },
  components: {
    MuiButton: {
      defaultProps: {
        disableRipple: true,
      },
      styleOverrides: {
        root: {
          minWidth: '120px',
          fontSize: '0.8rem',
          height: '40px',
          '&:hover': {
            boxShadow: 6,
          },
        },
      },
    },
  },
});

export default theme;
