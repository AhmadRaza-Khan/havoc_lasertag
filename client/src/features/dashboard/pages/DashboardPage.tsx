import { PageHeader } from '@/components/layout/PageHeader'
import { useAuth } from '@/features/auth'

export default function DashboardPage() {
  const { user } = useAuth()

  return (
    <>
      <PageHeader title="Dashboard" description={`Welcome back, ${user?.name}.`} />
      <div className="panel panel-padded">
        <p>Hello, world!</p>
      </div>
    </>
  )
}
