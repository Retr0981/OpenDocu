# opendocu-web

Next.js 14 web interface for OpenDocu.

## Quick start

```bash
npm install
npm run dev
```

Open http://localhost:3000 — you'll be redirected to `/home`.

## Pages

| Route        | Description                                              |
| ------------ | -------------------------------------------------------- |
| `/home`      | Feature showcase with a **live, client-side** demo       |
| `/upload`    | Drag-and-drop upload + processing options                |
| `/results`   | Side-by-side comparison with editable output + export    |
| `/dashboard` | Usage analytics and processing history                   |
| `/settings`  | Reduction preferences                                     |
| `/api-doc`   | Interactive REST API + language binding reference        |

## How the demo works

The live demo uses a dependency-free TypeScript port of the Rust extractive
summarizer (`lib/reduce.ts`). It runs entirely in the browser — no backend or
WASM required. This makes the demo fast and private.

For production-grade accuracy and performance, compile the Rust core to
WebAssembly (see `lib/wasm/README.md`) or call the native binding from a
Next.js API route.

## Stack

- **Next.js 14** (app router, server components)
- **React 18** (functional components, hooks)
- **Tailwind CSS** (with dark mode)
- **TypeScript** throughout
