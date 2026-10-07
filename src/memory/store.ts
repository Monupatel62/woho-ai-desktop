export interface MemoryRecord {
  id: string;
  namespace: string;
  content: string;
  createdAt: string;
}

export interface MemoryStore {
  put(record: MemoryRecord): Promise<void>;
  get(id: string): Promise<MemoryRecord | undefined>;
  delete(id: string): Promise<void>;
}

export function validateMemoryRecord(record: MemoryRecord): void {
  if (!record.id || !record.namespace || !record.createdAt) {
    throw new Error("Invalid memory record");
  }
}
