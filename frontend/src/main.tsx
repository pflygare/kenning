import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { createBrowserRouter, RouterProvider } from 'react-router'
import './index.css'
import AuthProvider from './auth/AuthProvider.tsx'
import RequireAuth from './auth/RequireAuth.tsx'
import ForgotPassword from './routes/ForgotPassword.tsx'
import Home from './routes/Home.tsx'
import Invite from './routes/Invite.tsx'
import Layout from './routes/Layout.tsx'
import Login from './routes/Login.tsx'
import Members from './routes/Members.tsx'
import NewOrg from './routes/NewOrg.tsx'
import NotFound from './routes/NotFound.tsx'
import OrgHome from './routes/OrgHome.tsx'
import OrgLayout from './routes/OrgLayout.tsx'
import ResetPassword from './routes/ResetPassword.tsx'
import Signup from './routes/Signup.tsx'
import VerifyEmail from './routes/VerifyEmail.tsx'

const router = createBrowserRouter([
  {
    element: <Layout />,
    children: [
      { index: true, element: <Home /> },
      { path: 'login', element: <Login /> },
      { path: 'signup', element: <Signup /> },
      { path: 'forgot-password', element: <ForgotPassword /> },
      { path: 'reset-password', element: <ResetPassword /> },
      { path: 'verify-email', element: <VerifyEmail /> },
      { path: 'invite/:token', element: <Invite /> },
      {
        path: 'new-org',
        element: (
          <RequireAuth>
            <NewOrg />
          </RequireAuth>
        ),
      },
      {
        // Organization slugs never collide with the paths above (the server reserves them).
        path: ':org',
        element: (
          <RequireAuth>
            <OrgLayout />
          </RequireAuth>
        ),
        children: [
          { index: true, element: <OrgHome /> },
          { path: 'settings/members', element: <Members /> },
        ],
      },
      { path: '*', element: <NotFound /> },
    ],
  },
])

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <AuthProvider>
      <RouterProvider router={router} />
    </AuthProvider>
  </StrictMode>,
)
