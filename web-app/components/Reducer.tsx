'use client';

import { useMemo, useState } from 'react';
import { reduce, ReductionLevel } from '@/lib/reduce';

const SAMPLE = `# OpenDocu

OpenDocu is a document reduction platform built in Rust. It processes documents quickly and supports many formats including PDF, DOCX, and Markdown files. Performance is a primary goal of the project, targeting very large files.

## Features

The core library provides summarization, key point extraction, and semantic compression. Users can choose between light, medium, and aggressive reduction levels. Streaming output enables real-time processing of documents as they are parsed.

## Architecture

The Rust workspace is organized into several focused crates with no cyclic dependencies. The parser crate handles format detection and text extraction from various sources. The summarizer crate implements extractive and abstractive summarization algorithms.`;

export default function Reducer() {
  const [text, setText] = useState(SAMPLE);
  const [level, setLevel] = useState<ReductionLevel>('medium');

  const result = useMemo(() => reduce(text, { level }), [text, level]);

  return (
    <div className="grid md:grid-cols-2 gap-6">
      <div>
        <label className="block text-sm font-medium mb-2">Input document</label>
        <textarea
          className="w-full h-64 p-3 rounded border border-gray-300 dark:border-gray-700 dark:bg-gray-900 font-mono text-sm"
          value={text}
          onChange={(e) => setText(e.target.value)}
        />
        <div className="mt-3 flex gap-2">
          {(['light', 'medium', 'aggressive'] as ReductionLevel[]).map((l) => (
            <button
              key={l}
              onClick={() => setLevel(l)}
              className={`px-3 py-1 rounded text-sm capitalize ${
                level === l
                  ? 'bg-brand-600 text-white'
                  : 'bg-gray-100 dark:bg-gray-800 text-gray-700 dark:text-gray-300'
              }`}
            >
              {l}
            </button>
          ))}
        </div>
      </div>

      <div className="space-y-4">
        <div className="rounded-lg bg-gray-50 dark:bg-gray-900 p-4">
          <div className="flex justify-between text-sm text-gray-500 mb-2">
            <span>Reduction</span>
            <span className="font-mono">{result.metrics.reductionPercent.toFixed(1)}%</span>
          </div>
          <div className="h-2 bg-gray-200 dark:bg-gray-700 rounded-full overflow-hidden">
            <div
              className="h-full bg-brand-500"
              style={{ width: `${Math.min(100, result.metrics.reductionPercent)}%` }}
            />
          </div>
          <div className="flex justify-between text-xs text-gray-400 mt-1">
            <span>{result.metrics.originalWords} words</span>
            <span>→ {result.metrics.reducedWords} words</span>
          </div>
        </div>

        <div>
          <h3 className="text-sm font-medium mb-2">Summary</h3>
          <p className="text-sm leading-relaxed">{result.summary || '—'}</p>
        </div>

        <div>
          <h3 className="text-sm font-medium mb-2">Key points</h3>
          <ul className="text-sm space-y-1 list-disc list-inside">
            {result.keyPoints.map((kp, i) => (
              <li key={i}>{kp}</li>
            ))}
          </ul>
        </div>

        <div>
          <h3 className="text-sm font-medium mb-2">Keywords</h3>
          <div className="flex flex-wrap gap-1">
            {result.keywords.map((k) => (
              <span
                key={k}
                className="px-2 py-0.5 text-xs rounded bg-brand-50 text-brand-700 dark:bg-brand-900 dark:text-brand-100"
              >
                {k}
              </span>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
