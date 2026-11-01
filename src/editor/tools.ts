/** Werkzeuge: Auswählen, Gleis platzieren, Löschen, Verschieben ... */
export interface Tool {
  name: string;
  onPointerDown(x: number, y: number): void;
  onPointerMove(x: number, y: number): void;
  onPointerUp(x: number, y: number): void;
}
