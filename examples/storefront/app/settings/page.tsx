'use client'

import { useState } from 'react'

import { Badge } from '@/components/ui/Badge'
import { Button } from '@/components/ui/Button'
import { Card } from '@/components/ui/Card'
import { TextField } from '@/components/ui/TextField'

export default function SettingsPage() {
  const [name, setName] = useState('')

  return (
    <main className="sf-page">
      <h1>Settings</h1>
      <Card title="Account" footer={<Badge>Draft</Badge>}>
        <TextField label="Display name" value={name} onChange={setName} />
        <TextField label="Support email" value="" onChange={() => undefined} disabled />
        <Button variant="secondary" size="sm" onClick={() => undefined}>
          Save
        </Button>
      </Card>
    </main>
  )
}
