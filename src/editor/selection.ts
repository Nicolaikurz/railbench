export class Selection {
  private ids = new Set<string>();
  set(id: string) { this.ids = new Set([id]); }
  setMany(ids: string[]) { this.ids = new Set(ids); }
  add(id: string) { this.ids.add(id); }
  addMany(ids: string[]) { for (const id of ids) this.ids.add(id); }
  toggle(id: string) { this.ids.has(id) ? this.ids.delete(id) : this.ids.add(id); }
  clear() { this.ids.clear(); }
  has(id: string) { return this.ids.has(id); }
  all() { return [...this.ids]; }
}
