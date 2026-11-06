import type { FileInfo, OperationCategory, OperationDescriptor } from "./types";
import { FORMAT_LABELS } from "./api";

export const CATEGORY_ORDER: OperationCategory[] = ["convert", "compress", "transform", "extract", "combine"];

type T = (key: string, params?: Record<string, string | number>) => string;

export function opLabel(t: T, op: OperationDescriptor): string {
  const format = op.output ? FORMAT_LABELS[op.output] ?? op.output.toUpperCase() : "";
  return t(op.labelKey, { format });
}

/** Group operations by category, in display order. */
export function groupByCategory(ops: OperationDescriptor[]) {
  return CATEGORY_ORDER.map((c) => [c, ops.filter((o) => o.category === c)] as const).filter(([, list]) => list.length > 0);
}

/** Operations offered by at least two of the files, with how many files support each. */
export function sharedOperations(files: FileInfo[]): { op: OperationDescriptor; count: number }[] {
  const byId = new Map<string, { op: OperationDescriptor; count: number }>();
  for (const f of files) {
    for (const op of f.operations) {
      const entry = byId.get(op.id);
      if (entry) entry.count++;
      else byId.set(op.id, { op, count: 1 });
    }
  }
  return [...byId.values()].filter((e) => e.count >= 2);
}

export const supports = (file: FileInfo, opId: string) => file.operations.some((o) => o.id === opId);
