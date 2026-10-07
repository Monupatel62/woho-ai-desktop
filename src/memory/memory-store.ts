export interface MemoryRecord { id: string; content: string; createdAt: number; }
export interface MemoryStore {
  put(record: MemoryRecord): Promise<void>;
  get(id: string): Promise<MemoryRecord | undefined>;
  delete(id: string): Promise<void>;
  list(): Promise<MemoryRecord[]>;
}
export class InMemoryStore implements MemoryStore {
  private readonly records = new Map<string, MemoryRecord>();
  async put(record: MemoryRecord): Promise<void> {
    if (!/^[a-z0-9][a-z0-9._-]{0,127}$/.test(record.id)) throw new Error("Invalid memory id");
    if (!record.content || record.content.length > 64 * 1024) throw new Error("Invalid memory content");
    this.records.set(record.id, { ...record });
  }
  async get(id: string) { const value = this.records.get(id); return value ? { ...value } : undefined; }
  async delete(id: string) { this.records.delete(id); }
  async list() { return [...this.records.values()].map((value) => ({ ...value })); }
}
