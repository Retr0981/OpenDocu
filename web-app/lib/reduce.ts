/**
 * Dependency-free JS port of OpenDocu's extractive summarizer.
 *
 * This is a browser-friendly reimplementation of the Rust TextRank/LexRank
 * algorithm in `crates/summarizer`. It enables the web demo to run entirely
 * client-side without WASM or a backend. For production accuracy and speed,
 * prefer the native binding (@opendocu/node) — this port targets clarity.
 *
 * Pipeline:
 *   1. Split text into sentences.
 *   2. Tokenize (lowercase, strip stopwords, light stem).
 *   3. Build a TF-IDF cosine-similarity matrix between sentences.
 *   4. Run PageRank to score sentences.
 *   5. Pick the top-N and return them in original order.
 */

export type ReductionLevel = 'light' | 'medium' | 'aggressive';

export interface ProcessingOptions {
  level?: ReductionLevel;
  maxSentences?: number;
  maxKeyPoints?: number;
  maxKeywords?: number;
  minSentenceWords?: number;
}

export interface ReducedDocument {
  title: string | null;
  summary: string;
  keyPoints: string[];
  keywords: string[];
  metrics: {
    originalWords: number;
    reducedWords: number;
    reductionPercent: number;
    originalSentences: number;
  };
}

const STOP_WORDS = new Set<string>([
  'a', 'an', 'the', 'and', 'or', 'but', 'is', 'are', 'was', 'were', 'be', 'been',
  'being', 'have', 'has', 'had', 'do', 'does', 'did', 'will', 'would', 'could',
  'should', 'may', 'might', 'must', 'shall', 'can', 'need', 'dare', 'ought',
  'used', 'to', 'of', 'in', 'for', 'on', 'with', 'at', 'by', 'from', 'as',
  'into', 'through', 'during', 'before', 'after', 'above', 'below', 'between',
  'this', 'that', 'these', 'those', 'i', 'you', 'he', 'she', 'it', 'we', 'they',
  'what', 'which', 'who', 'whom', 'whose', 'when', 'where', 'why', 'how', 'all',
  'each', 'every', 'both', 'few', 'more', 'most', 'other', 'some', 'such', 'no',
  'not', 'only', 'own', 'same', 'so', 'than', 'too', 'very', 'just', 'also',
]);

const RETENTION: Record<ReductionLevel, number> = {
  light: 0.6,
  medium: 0.35,
  aggressive: 0.15,
};

function lightStem(word: string): string {
  const suffixes = ['ations', 'ation', 'ization', 'izing', 'ized', 'ness', 'tion',
    'ions', 'ing', 'ied', 'ies', 'ed', 'er', 'est', 'ly', 's'];
  for (const s of suffixes) {
    if (word.endsWith(s) && word.length > s.length + 2) {
      return word.slice(0, word.length - s.length);
    }
  }
  return word;
}

function tokenize(text: string): string[] {
  return text
    .toLowerCase()
    .split(/[^a-z0-9]+/)
    .filter((w) => w.length > 2 && !STOP_WORDS.has(w))
    .map(lightStem);
}

function splitSentences(text: string): string[] {
  // crude but effective: split on sentence-final punctuation followed by space.
  return text
    .replace(/\s+/g, ' ')
    .trim()
    .split(/(?<=[.!?])\s+/)
    .map((s) => s.trim())
    .filter((s) => s.length > 0);
}

function cosineSim(a: Map<string, number>, b: Map<string, number>): number {
  let dot = 0;
  let magA = 0;
  let magB = 0;
  for (const [k, v] of a) {
    magA += v * v;
    if (b.has(k)) dot += v * b.get(k)!;
  }
  for (const v of b.values()) magB += v * v;
  if (magA === 0 || magB === 0) return 0;
  return dot / (Math.sqrt(magA) * Math.sqrt(magB));
}

function pageRank(matrix: number[][], damping = 0.85, iterations = 60): number[] {
  const n = matrix.length;
  if (n === 0) return [];
  let scores = new Array(n).fill(1 / n);
  const teleport = (1 - damping) / n;

  for (let _ = 0; _ < iterations; _++) {
    const next = new Array(n).fill(teleport);
    for (let j = 0; j < n; j++) {
      let acc = 0;
      for (let i = 0; i < n; i++) {
        if (matrix[i][j] > 0) acc += damping * matrix[i][j] * scores[i];
      }
      next[j] += acc;
    }
    scores = next;
  }
  return scores;
}

/**
 * Reduce a document. The main entry point used by the web demo.
 */
export function reduce(input: string, options: ProcessingOptions = {}): ReducedDocument {
  const level = options.level ?? 'medium';
  const minWords = options.minSentenceWords ?? 4;
  const retention = RETENTION[level];

  // Detect a title (first markdown # heading or first line).
  let title: string | null = null;
  const titleMatch = input.match(/^#\s+(.+)$/m);
  if (titleMatch) title = titleMatch[1].trim();

  // Strip simple markdown for sentence splitting.
  const stripped = input.replace(/^#+\s+/gm, '').replace(/[*`_>]/g, '');
  const sentences = splitSentences(stripped);

  const originalWords = stripped.split(/\s+/).filter(Boolean).length;

  if (sentences.length === 0) {
    return {
      title,
      summary: '',
      keyPoints: [],
      keywords: [],
      metrics: { originalWords, reducedWords: 0, reductionPercent: 0, originalSentences: 0 },
    };
  }

  // TF vectors per sentence (stemmed).
  const tfs = sentences.map((s) => {
    const m = new Map<string, number>();
    for (const t of tokenize(s)) m.set(t, (m.get(t) ?? 0) + 1);
    return m;
  });

  // Build similarity matrix and row-normalize.
  const n = sentences.length;
  const matrix: number[][] = Array.from({ length: n }, () => new Array(n).fill(0));
  for (let i = 0; i < n; i++) {
    for (let j = i + 1; j < n; j++) {
      const sim = cosineSim(tfs[i], tfs[j]);
      if (sim > 0) {
        matrix[i][j] = sim;
        matrix[j][i] = sim;
      }
    }
    let rowSum = 0;
    for (let j = 0; j < n; j++) rowSum += matrix[i][j];
    if (rowSum > 0) {
      for (let j = 0; j < n; j++) matrix[i][j] /= rowSum;
    }
  }

  const scores = pageRank(matrix);

  // Pick top-N sentences.
  const target = options.maxSentences ?? Math.max(1, Math.round(n * retention));
  const order = scores
    .map((s, i) => ({ s, i, wc: sentences[i].split(/\s+/).length }))
    .filter((x) => x.wc >= minWords || scores.every((sc) => sc === scores[0]));
  order.sort((a, b) => b.s - a.s);
  const picked = order.slice(0, target).map((x) => x.i).sort((a, b) => a - b);
  const summary = picked.map((i) => sentences[i]).join(' ');

  // Key points: combine salience (0.7) with position bonus (0.3).
  const maxKp = options.maxKeyPoints ?? (level === 'light' ? 3 : level === 'medium' ? 5 : 7);
  const maxScore = Math.max(...scores, 1e-9);
  const kpRanked = scores
    .map((s, i) => ({
      combined: 0.7 * (s / maxScore) + 0.3 * (1 - i / Math.max(1, n)),
      i,
      wc: sentences[i].split(/\s+/).length,
    }))
    .filter((x) => x.wc >= minWords)
    .sort((a, b) => b.combined - a.combined)
    .slice(0, maxKp)
    .sort((a, b) => a.i - b.i)
    .map((x) => sentences[x.i]);

  // Keywords: top TF across the whole doc.
  const maxKw = options.maxKeywords ?? (level === 'light' ? 5 : level === 'medium' ? 8 : 12);
  const globalTf = new Map<string, number>();
  for (const s of sentences) {
    for (const t of tokenize(s)) globalTf.set(t, (globalTf.get(t) ?? 0) + 1);
  }
  const keywords = [...globalTf.entries()]
    .sort((a, b) => b[1] - a[1])
    .slice(0, maxKw)
    .map(([k]) => k);

  const reducedWords = summary.split(/\s+/).filter(Boolean).length;
  const reductionPercent = originalWords > 0
    ? (1 - reducedWords / originalWords) * 100
    : 0;

  return {
    title,
    summary,
    keyPoints: kpRanked,
    keywords,
    metrics: {
      originalWords,
      reducedWords,
      reductionPercent,
      originalSentences: n,
    },
  };
}
