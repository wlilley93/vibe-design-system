'use client'

import { useState } from 'react'

import { Alert } from '@/components/ui/Alert'
import { Button } from '@/components/ui/Button'
import { Card } from '@/components/ui/Card'
import { TextField } from '@/components/ui/TextField'

export default function CheckoutPage() {
  const [postcode, setPostcode] = useState('')
  const invalid = postcode.length > 0 && postcode.length < 5

  return (
    <main className="sf-page">
      <h1>Checkout</h1>
      {invalid ? (
        <Alert title="Check the postcode" tone="critical">
          A postcode is at least five characters.
        </Alert>
      ) : null}
      <Card title="Delivery">
        <TextField
          label="Postcode"
          value={postcode}
          onChange={setPostcode}
          invalid={invalid}
        />
        <Button disabled={invalid} onClick={() => undefined}>
          Continue
        </Button>
      </Card>
    </main>
  )
}
