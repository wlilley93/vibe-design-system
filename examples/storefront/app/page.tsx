import { Badge } from '@/components/ui/Badge'
import { Button } from '@/components/ui/Button'
import { Card } from '@/components/ui/Card'

export default function HomePage() {
  return (
    <main className="sf-page">
      <h1>Storefront</h1>
      <Card title="Today" footer={<Button variant="ghost">See all</Button>}>
        <p>
          Three orders are waiting. <Badge tone="positive">On time</Badge>
        </p>
        <Button onClick={() => undefined}>Open the queue</Button>
      </Card>
      <Card title="Stock">
        <p>
          One line is short. <Badge tone="critical">Reorder</Badge>
        </p>
      </Card>
    </main>
  )
}
