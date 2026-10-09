import { Link } from 'react-router'
import Page from '../components/Page'

export default function NotFound() {
  return (
    <Page narrow>
      <div className="card empty">
        <h2>Page not found</h2>
        <p>There is nothing at this address, or you don't have access to it.</p>
        <Link className="button" to="/">
          Go home
        </Link>
      </div>
    </Page>
  )
}
