'use client';

import { useState } from 'react';
import { useRouter } from 'next/navigation';
import Reducer from '@/components/Reducer';

export default function UploadPage() {
  const [dragOver, setDragOver] = useState(false);
  const [text, setText] = useState('');
  const router = useRouter();

  const handleFile = async (file: File) => {
    // Only handle text files client-side; binary formats would need the server.
    if (file.type.startsWith('text/') || /\.(md|txt|markdown|html)$/i.test(file.name)) {
      const content = await file.text();
      setText(content);
    } else {
      alert(
        `Client-side demo supports text formats only (${file.name}). ` +
          'Binary parsing (PDF/DOCX/EPUB) runs via the native binding on the server.'
      );
    }
  };

  return (
    <div className="space-y-8">
      <header>
        <h1 className="text-3xl font-bold">Upload a document</h1>
        <p className="text-gray-600 dark:text-gray-400 mt-1">
          Drag and drop a text file, or paste content below. Processing happens in your browser.
        </p>
      </header>

      <div
        onDragOver={(e) => {
          e.preventDefault();
          setDragOver(true);
        }}
        onDragLeave={() => setDragOver(false)}
        onDrop={(e) => {
          e.preventDefault();
          setDragOver(false);
          const file = e.dataTransfer.files[0];
          if (file) handleFile(file);
        }}
        className={`border-2 border-dashed rounded-lg p-12 text-center cursor-pointer transition ${
          dragOver
            ? 'border-brand-500 bg-brand-50 dark:bg-brand-950'
            : 'border-gray-300 dark:border-gray-700'
        }`}
        onClick={() => document.getElementById('file-input')?.click()}
      >
        <input
          id="file-input"
          type="file"
          className="hidden"
          accept=".txt,.md,.markdown,.html,.pdf,.docx,.epub"
          onChange={(e) => {
            const f = e.target.files?.[0];
            if (f) handleFile(f);
          }}
        />
        <p className="text-gray-500">
          {text ? `Loaded ${text.split(/\s+/).length} words` : 'Drop a file here or click to browse'}
        </p>
      </div>

      <Reducer />

      <button
        onClick={() => router.push('/results')}
        className="px-4 py-2 rounded bg-brand-600 text-white"
      >
        View results
      </button>
    </div>
  );
}
