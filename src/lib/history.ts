// Conversion history: the last 100 finished actions, kept in this PC's webview storage
// (never sent anywhere). Survives restarts; cleared from the History panel.

export interface HistoryEntry {
  /** ms since 1970. */
  at: number;
  operationId: string;
  /** Action label in the language used at the time ("En PNG"). */
  label: string;
  outputs: string[];
  done: number;
  failed: number;
}

const KEY = "convix.history";
const MAX = 100;
type Listener = (entries: HistoryEntry[]) => void;
const listeners = new Set<Listener>();

export function loadHistory(): HistoryEntry[] {
  try {
    const raw = JSON.parse(localStorage.getItem(KEY) ?? "[]");
    return Array.isArray(raw) ? (raw as HistoryEntry[]).filter((e) => e && Array.isArray(e.outputs)) : [];
  } catch {
    return [];
  }
}

function store(entries: HistoryEntry[]) {
  try {
    localStorage.setItem(KEY, JSON.stringify(entries));
  } catch {
    /* storage full or unavailable: history is a convenience */
  }
  listeners.forEach((l) => l(entries));
}

export function addHistory(entry: HistoryEntry) {
  if (entry.done === 0 && entry.failed === 0) return; // cancelled: nothing worth keeping
  store([entry, ...loadHistory()].slice(0, MAX));
}

export function clearHistory() {
  store([]);
}

export function onHistoryChange(listener: Listener): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}
