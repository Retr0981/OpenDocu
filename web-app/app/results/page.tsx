'use client';

import { useState } from 'react';
import { reduce, ReductionLevel } from '@/lib/reduce';

const SAMPLE = `# Quarterly Report

Revenue increased 23% year over year driven by strong enterprise adoption. The platform processed over two million documents during the quarter. Customer retention reached 94%, an all-time high for the company.

Operating expenses grew modestly as we expanded the engineering team. Research and development investment focused on the Rust core and WebAssembly bindings. Marketing spend was concentrated on developer relations and documentation.

Cash position remains strong with eighteen months of runway. The board approved an expansion into European markets beginning next quarter. We expect regulatory compliance work to conclude by year end.`;

export default function ResultsPage() {
  const [text, setText] = useState(SAMPLE);
  const [level, setLevel] = useState<ReductionLevel>('medium');
  const result = reduce(text, { level });
  const [edited, setEdited] = useState('');

  return (
    <div className="space-y-6">
      <header>
        <h1 className="text-3xl font-bold">Results</h1>
        <p className="text-gray-600 dark:text-gray-400 mt-1">
          Side-by-side comparison with editable output. Export to TXT, MD, or JSON.
        </p>
      </header>

      <div className="flex gap-2">
        {(['light', 'medium', 'aggressive'] as ReductionLevel[]).map((l) => (
          <button
            key={l}
            onClick={() => setLevel(l)}
            className={`px-3 py-1 rounded text-sm capitalize ${
              level === l ? 'bg-brand-600 text-white' : 'bg-gray-100 dark:bg-gray-800'
            }`}
          >
            {l}
          </button>
        ))}
      </div>

      <div className="grid md:grid-cols-2 gap-6">
        <div>
          <div className="flex justify-between items-center mb-2">
            <h2 className="font-medium">Original</h2>
            <span className="text-xs text-gray-500">{result.metrics.originalWords} words</span>
          </div>
          <textarea
            className="w-full h-80 p-3 rounded border border-gray-300 dark:border-gray-700 dark:bg-gray-900 text-sm font-mono"
            value={text}
            onChange={(e) => setText(e.target.value)}
          />
        </div>

        <div>
          <div className="flex justify-between items-center mb-2">
            <h2 className="font-medium">Reduced</h2>
            <span className="text-xs text-gray-500">
              {result.metrics.reducedWords} words · -{result.metrics.reductionPercent.toFixed(0)}%
            </span>
          </div>
          <textarea
            className="w-full h-80 p-3 rounded border border-brand-200 dark:border-brand-800 dark:bg-gray-900 text-sm font-mono"
            value={edited || result.summary}
            onChange={(e) => setEdited(e.target.value)}
            placeholder={result.summary}
          />
        </div>
      </div>

      <div className="flex gap-2">
        {[
          { fmt: 'TXT', content: result.summary },
          { fmt: 'MD', content: `# ${result.title ?? 'Summary'}\n\n${result.summary}\n\n## Key points\n${result.keyPoints.map((p) => `- ${p}`).join('\n')}` },
          { fmt: 'JSON', content: JSON.stringify(result, null, 2) },
        ].map((opt) => (
          <button
            key={opt.fmt}
            onClick={() => {
              const blob = new Blob([opt.content], { type: 'text/plain' });
              const url = URL.createObjectURL(blob);
              const a = document.createElement('a');
              a.href = url;
              a.download = `opendocu-result.${opt.fmt.toLowerCase()}`;
              a.click();
              URL.revokeObjectURL(url);
            }}
            className="px-3 py-1 rounded border border-gray-300 dark:border-gray-700 text-sm"
          >
            Export {opt.fmt}
          </button>
        ))}
      </div>

      {result.keyPoints.length > 0 && (
        <div>
          <h2 className="font-medium mb-2">Key points</h2>
          <ul className="list-disc list-inside text-sm space-y-1">
            {result.keyPoints.map((p, i) => (
              <li key={i}>{p}</li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}
