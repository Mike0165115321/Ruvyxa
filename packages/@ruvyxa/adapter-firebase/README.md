# @ruvyxa/adapter-firebase

Full-stack Firebase Hosting adapter for Ruvyxa. It publishes static assets to Firebase's CDN and
rewrites dynamic requests to a generated second-generation HTTPS function.

```bash
ruvyxa build --adapter firebase
firebase deploy --only hosting,functions
```

The build creates `firebase.json` without overwriting an existing file. Firebase project selection
and authentication remain Firebase CLI responsibilities. SSR, SSG, CSR, ISR, PPR, and API routes are
supported; native WebSocket realtime requires a long-lived Node/Bun host instead.
