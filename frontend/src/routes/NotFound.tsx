import { Link } from 'react-router'

export default function NotFound() {
  return (
    <>
      <h1>Page not found</h1>
      <p>
        There is nothing at this address. <Link to="/">Go home</Link>
      </p>
    </>
  )
}
