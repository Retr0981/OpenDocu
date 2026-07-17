'use client';

import { useState } from 'react';

export default function SettingsPage() {
  const [level, setLevel] = useState('medium');
  const [outputFormat, setOutputFormat] = useState('markdown');
  const [abstractive, setAbstractive] = useState(false);
  const [minSentenceWords, setMinSentenceWords] = useState(4);

  return (
    <div className="max-w-2xl space-y-8">
      <header>
        <h1 className="text-3xl font-bold">Settings</h1>
        <p className="text-gray-600 dark:text-gray-400 mt-1">Customize reduction preferences.</p>
      </header>

      <section className="space-y-4">
        <div>
          <label className="block font-medium mb-2">Default reduction level</label>
          <div className="flex gap-2">
            {['light', 'medium', 'aggressive'].map((l) => (
              <button
                key={l}
                onClick={() => setLevel(l)}
                className={`px-4 py-2 rounded text-sm capitalize ${
                  level === l ? 'bg-brand-600 text-white' : 'bg-gray-100 dark:bg-gray-800'
                }`}
              >
                {l}
              </button>
            ))}
          </div>
          <p className="text-xs text-gray-500 mt-1">
            Light keeps ~60%, medium ~35%, aggressive ~15% of the original.
          </p>
        </div>

        <div>
          <label className="block font-medium mb-2">Output format</label>
          <select
            value={outputFormat}
            onChange={(e) => setOutputFormat(e.target.value)}
            className="w-full p-2 rounded border border-gray-300 dark:border-gray-700 dark:bg-gray-900"
          >
            <option value="plaintext">Plain text</option>
            <option value="markdown">Markdown</option>
            <option value="json">JSON</option>
          </select>
        </div>

        <div>
          <label className="block font-medium mb-2">
            Minimum sentence length: {minSentenceWords} words
          </label>
          <input
            type="range"
            min={1}
            max={15}
            value={minSentenceWords}
            onChange={(e) => setMinSentenceWords(Number(e.target.value))}
            className="w-full"
          />
        </div>

        <div className="flex items-center gap-2">
          <input
            id="abstractive"
            type="checkbox"
            checked={abstractive}
            onChange={(e) => setAbstractive(e.target.checked)}
          />
          <label htmlFor="abstractive" className="text-sm">
            Enable abstractive pass (local, key-less provider)
          </label>
        </div>
      </section>

      <button className="px-4 py-2 rounded bg-brand-600 text-white">Save preferences</button>
    </div>
  );
}
