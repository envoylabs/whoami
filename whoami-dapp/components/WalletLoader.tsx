import { ReactNode } from 'react'
import { useSigningClient } from 'contexts/cosmwasm'
import Loader from './Loader'
import { isKeplrInstalled } from 'services/keplr'


function WalletLoader({
  children,
  loading = false,
}: {
  children: ReactNode
  loading?: boolean
}) {
  const {
    walletAddress,
    loading: clientLoading,
    error,
    connectWallet,
  } = useSigningClient()

  if (loading || clientLoading) {
    return (
      <div className="flex justify-center">
        <Loader />
      </div>
    )
  }

  if (walletAddress === '') {
      const keplrInstalled = isKeplrInstalled()
      const actionText = keplrInstalled ?
			 (<>
			     <p>Please connect your Keplr wallet to continue</p>
			 </>)
			 :
			 (<>
			     <p>Please install the <a href="https://keplr.app" className="link">Keplr wallet</a> to continue</p>
			     <p>Once you've finished installation reload this page</p>
			 </>)
      const actionButton = keplrInstalled ?
			   (
			       <button className="btn btn-primary" onClick={connectWallet}>
				   <h3>Connect your wallet</h3>
			       </button>
			   )
			 : (
			     <a href="https://keplr.app" className='btn btn-primary'>
				 <h3>GetKeplr</h3>
			     </a>
			 )
      return (
	  <>
	      {children}
	      <div className="modal modal-open">
		  <div className="modal-box">
		      {actionText}
		      <div className="modal-action justify-center">
			  {actionButton}
		      </div>
		  </div>
	      </div>
	  </>
      )
  }

  if (error) {
    return <code>{JSON.stringify(error)}</code>
  }

  return <>{children}</>
}

export default WalletLoader
