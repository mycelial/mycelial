import './globals.css'
import 'reactflow/dist/style.css'
import { Inter } from 'next/font/google'

import { MycMantineProvider } from '@/components/MantineProvider';

const inter = Inter({ subsets: ['latin'] })

export const metadata = {
  title: 'Mycelial Console',
  description: '',
}

export default function RootLayout({
  children,
}: {
  children: React.ReactNode
}) {
  return (
    <html lang="en">
      <MycMantineProvider>
        <body className={inter.className}>{children}</body>
      </MycMantineProvider>
    </html>
  )
}
