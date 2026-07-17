const stats = [
  { label: 'Documents processed', value: '12,847', delta: '+8.2%' },
  { label: 'Words reduced', value: '4.2M', delta: '+12.1%' },
  { label: 'Avg. reduction', value: '67%', delta: '+1.4%' },
  { label: 'Avg. latency', value: '94ms', delta: '-3.0%' },
];

// Mock recent activity — in production this comes from the SQLite store.
const history = [
  { name: 'quarterly-report.pdf', format: 'PDF', reduction: '71%', when: '2 min ago' },
  { name: 'research-notes.md', format: 'MD', reduction: '58%', when: '1 hour ago' },
  { name: 'meeting-transcript.docx', format: 'DOCX', reduction: '64%', when: '3 hours ago' },
  { name: 'api-spec.html', format: 'HTML', reduction: '49%', when: 'yesterday' },
  { name: 'manual.epub', format: 'EPUB', reduction: '77%', when: '2 days ago' },
];

export default function DashboardPage() {
  return (
    <div className="space-y-8">
      <header>
        <h1 className="text-3xl font-bold">Dashboard</h1>
        <p className="text-gray-600 dark:text-gray-400 mt-1">Usage analytics and processing history.</p>
      </header>

      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        {stats.map((s) => (
          <div key={s.label} className="p-5 rounded-lg border border-gray-200 dark:border-gray-800">
            <div className="text-2xl font-bold">{s.value}</div>
            <div className="text-sm text-gray-500">{s.label}</div>
            <div
              className={`text-xs mt-1 ${s.delta.startsWith('-') ? 'text-red-500' : 'text-green-500'}`}
            >
              {s.delta} vs last week
            </div>
          </div>
        ))}
      </div>

      <div>
        <h2 className="font-medium mb-3">Recent documents</h2>
        <div className="rounded-lg border border-gray-200 dark:border-gray-800 overflow-hidden">
          <table className="w-full text-sm">
            <thead className="bg-gray-50 dark:bg-gray-900 text-left text-gray-500">
              <tr>
                <th className="px-4 py-2">Document</th>
                <th className="px-4 py-2">Format</th>
                <th className="px-4 py-2">Reduction</th>
                <th className="px-4 py-2">When</th>
              </tr>
            </thead>
            <tbody>
              {history.map((h) => (
                <tr key={h.name} className="border-t border-gray-100 dark:border-gray-800">
                  <td className="px-4 py-2 font-mono">{h.name}</td>
                  <td className="px-4 py-2">{h.format}</td>
                  <td className="px-4 py-2">{h.reduction}</td>
                  <td className="px-4 py-2 text-gray-500">{h.when}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}
