const endpoints = [
  {
    method: 'POST',
    path: '/v1/reduce',
    body: '{ "input": "<base64 or utf-8>", "level": "medium" }',
    response: '{ "summary": "...", "key_points": [...], "metrics": {...} }',
  },
  {
    method: 'POST',
    path: '/v1/reduce/stream',
    body: 'text/event-stream of ProgressEvent → Done',
    response: 'Same as /v1/reduce in the final Done event',
  },
  {
    method: 'POST',
    path: '/v1/batch',
    body: '{ "items": ["<base64>", ...], "level": "aggressive" }',
    response: '{ "results": [ {...}, ... ] }',
  },
  {
    method: 'GET',
    path: '/v1/health',
    body: '—',
    response: '{ "status": "ok", "version": "0.1.0" }',
  },
];

const codeSamples = [
  {
    lang: 'JavaScript',
    code: `import { reduce } from '@opendocu/node';
const result = await reduce(buffer, { level: 'medium' });
console.log(result.summary);`,
  },
  {
    lang: 'Python',
    code: `import opendocu
result = opendocu.reduce(text, level='aggressive')
print(result['summary'])`,
  },
  {
    lang: 'Rust',
    code: `use opendocu_core::{reduce, ProcessingOptions};
let r = reduce(bytes, ProcessingOptions::default())?;
println!("{}", r.summary);`,
  },
  {
    lang: 'C++',
    code: `opendocu::reducer r;
auto result = r.reduce(text);
std::cout << result.summary;`,
  },
];

export default function ApiDocPage() {
  return (
    <div className="space-y-10">
      <header>
        <h1 className="text-3xl font-bold">API reference</h1>
        <p className="text-gray-600 dark:text-gray-400 mt-1">
          Interactive documentation for the OpenDocu REST API and language bindings.
        </p>
      </header>

      <section>
        <h2 className="text-xl font-semibold mb-3">REST endpoints</h2>
        <div className="space-y-3">
          {endpoints.map((e) => (
            <div
              key={e.path}
              className="rounded-lg border border-gray-200 dark:border-gray-800 p-4"
            >
              <div className="flex items-center gap-2 mb-2">
                <span
                  className={`px-2 py-0.5 text-xs rounded font-mono ${
                    e.method === 'GET'
                      ? 'bg-green-100 text-green-700'
                      : 'bg-blue-100 text-blue-700'
                  }`}
                >
                  {e.method}
                </span>
                <code className="font-mono text-sm">{e.path}</code>
              </div>
              <div className="text-xs space-y-1 text-gray-600 dark:text-gray-400">
                <div>
                  <span className="font-medium">Request:</span> <code>{e.body}</code>
                </div>
                <div>
                  <span className="font-medium">Response:</span> <code>{e.response}</code>
                </div>
              </div>
            </div>
          ))}
        </div>
      </section>

      <section>
        <h2 className="text-xl font-semibold mb-3">Language bindings</h2>
        <div className="grid md:grid-cols-2 gap-4">
          {codeSamples.map((c) => (
            <div
              key={c.lang}
              className="rounded-lg border border-gray-200 dark:border-gray-800 overflow-hidden"
            >
              <div className="px-3 py-1.5 text-xs font-medium bg-gray-50 dark:bg-gray-900">
                {c.lang}
              </div>
              <pre className="p-3 text-xs overflow-x-auto">
                <code>{c.code}</code>
              </pre>
            </div>
          ))}
        </div>
      </section>
    </div>
  );
}
