"use client";

import { useState } from "react";

import {
  MantineProvider,
  ColorScheme,
  ColorSchemeProvider,
} from "@mantine/core";

export function MycMantineProvider({ children }: { children: React.ReactNode }) {
  return (
    <MantineProvider
      theme={{
        globalStyles: (theme) => ({
          '*, *::before, *::after': {
            boxSizing: 'border-box',
          },

          body: {
            ...theme.fn.fontStyles(),
            backgroundColor: theme.colors.night[0],
            color: theme.colors.stem[0],
          },
        }),
        "colors": {
          "night": ["#192831", "#314958", "#456376"],
          "toadstool": ["#FC1717", "#FF6363", "#FF9A9A"],
          "forest": ["#293B35", "#3A554C", "#52776B"],
          "moss": ["#97B398", "#A8C6A9", "#B6D7B8"],
          "stem": ["#FEF1DD", "#FFEBCC", "#FFE1B4"]
        }

      }}
      withGlobalStyles
      withNormalizeCSS
    >
      {children}
    </MantineProvider>
  );
}