import Reducer from '@/components/Reducer';
import Link from 'next/link';

const features = [
  { title: 'Multi-format', body: 'PDF, DOCX, TXT, MD, HTML, EPUB — feature-gated in Rust.' },
  { title: 'Extractive + Abstractive', body: 'TextRank extraction with a pluggable LLM trait.' },
  { title: 'Streaming', body: 'Real-time progress events via tokio streams.' },
  { title: 'Three bindings', body: 'Node, Python, and C++ over one C ABI.' },
  { title: 'Parallel batch', body: 'rayon-powered batch reduction across cores.' },
  { title: 'Structure-aware', body: 'Headings, lists, and code blocks are preserved.' },
];

export default function HomePage() {
  return (
    <div className="space-y-16">
      <section className="text-center py-12">
        <h1 className="text-5xl font-bold tracking-tight">
          Reduce documents. <span className="text-brand-600">Keep the meaning.</span>
        </h1>
        <p className="mt-4 text-lg text-gray-600 dark:text-gray-400 max-w-2xl mx-auto">
          OpenDocu is a high-performance document reduction platform built in Rust.
          Summarize, condense, and extract key information from any text — live, in your browser.
        </p>
        <div className="mt-6 flex justify-center gap-3">
          <Link
            href="/upload"
            className="px-5 py-2 rounded bg-brand-600 text-white hover:bg-brand-700"
          >
            Try it now
          </Link>
          <Link
            href="/api-doc"
            className="px-5 py-2 rounded border border-gray-300 dark:border-gray-700"
          >
            View the API
          </Link>
        </div>
      </section>

      <section>
        <h2 className="text-2xl font-semibold mb-6 text-center">Live demo</h2>
        <p className="text-sm text-gray-500 text-center mb-6">
          This demo runs the extractive summarizer entirely client-side — a JS port of the Rust
          algorithm. No data leaves your browser.
        </p>
        <Reducer />
      </section>

      <section>
        <h2 className="text-2xl font-semibold mb-6 text-center">Features</h2>
        <div className="grid md:grid-cols-3 gap-4">
          {features.map((f) => (
            <div
              key={f.title}
              className="p-5 rounded-lg border border-gray-200 dark:border-gray-800"
            >
              <h3 className="font-medium text-brand-600">{f.title}</h3>
              <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">{f.body}</p>
            </div>
          ))}
        </div>
      </section>
    </div>
  );
}
