import type { Annotation } from "./types";

export class AnnotationHistory {
  private states: Annotation[][];
  private index = 0;

  constructor(initial: Annotation[] = []) {
    this.states = [initial.slice()];
  }

  get current(): Annotation[] {
    return this.states[this.index].slice();
  }

  get canUndo(): boolean {
    return this.index > 0;
  }

  get canRedo(): boolean {
    return this.index < this.states.length - 1;
  }

  get size(): number {
    return this.states[this.index].length;
  }

  push(next: Annotation[]): Annotation[] {
    this.states = this.states.slice(0, this.index + 1);
    this.states.push(next.slice());
    this.index = this.states.length - 1;
    return this.current;
  }

  undo(): Annotation[] | null {
    if (!this.canUndo) return null;
    this.index -= 1;
    return this.current;
  }

  redo(): Annotation[] | null {
    if (!this.canRedo) return null;
    this.index += 1;
    return this.current;
  }

  reset(initial: Annotation[] = []): Annotation[] {
    this.states = [initial.slice()];
    this.index = 0;
    return this.current;
  }
}
