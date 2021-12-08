import { ReactNode } from 'react'
import Head from 'next/head'
import Nav from './Nav'

const POWERED_BY_URL="https://junonetwork.io"

export default function Layout({ children }: { children: ReactNode }) {
  return (
      <div className="flex flex-col items-center justify-center min-h-screen bg-base-100 text-base-content">
	  <Head>
              <title>{process.env.NEXT_PUBLIC_SITE_TITLE}</title>
              <meta name="description" content="Juno Name Service" />
              <link rel="icon" href="/favicon.ico" />
	  </Head>

	  <Nav />
	  <main className="flex flex-col items-center justify-center w-full flex-1 p-2 md:px-20 text-center">
              {children}
	  </main>
	  <footer className="flex items-center justify-center w-full h-24 border-t">
              Powered by{' '}
	      <a
		  className="pl-1 link link-primary link-hover"
		  href={POWERED_BY_URL}
              >
		  <div>Juno</div>
		  </a>
	  </footer>
      </div>
  )
}
