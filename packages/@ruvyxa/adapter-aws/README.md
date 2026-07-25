# @ruvyxa/adapter-aws

Full-stack AWS adapter targeting the official Amplify Hosting deployment specification. Amplify
builds auto-select it through `AWS_APP_ID` and receive `.amplify-hosting/static`,
`.amplify-hosting/compute/default`, and `deploy-manifest.json`.

```ts
import { awsAdapter } from '@ruvyxa/adapter-aws'
import { config } from 'ruvyxa/config'

export default config({ adapter: awsAdapter() })
```

The compute server listens on Amplify's required port 3000, stores runtime ISR refreshes under
`/tmp`, and supports SSR, SSG, CSR, ISR, PPR, and API routes. AWS credentials and Amplify app
creation remain AWS responsibilities.
