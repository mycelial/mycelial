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
      withGlobalStyles
      withNormalizeCSS
    >
      {children}
    </MantineProvider>
  );
}