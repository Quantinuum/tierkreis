import { createFileRoute } from '@tanstack/react-router'

export const Route = createFileRoute('/workflows/_/$wid/')({
  component: RouteComponent,
})

function RouteComponent() {
  return <div>Hello "/workflows/_/$wid/"!</div>
}
