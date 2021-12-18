import '../styles/globals.css'
import type { AppProps } from 'next/app'
import { ChakraProvider } from '@chakra-ui/react'
import { SWRConfig } from 'swr'

function SatysfiPlayground({ Component, pageProps }: AppProps) {
  return (
    <SWRConfig value={{
      fetcher: (input, init?) => fetch(input, init).then(response => response.text())
    }}>
      <ChakraProvider>
        <Component {...pageProps} />
      </ChakraProvider>
    </SWRConfig>
  )
}

export default SatysfiPlayground
