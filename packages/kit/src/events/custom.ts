export interface CustomEvent {
  targetId: number;
  name: string;
  body?: Record<string, unknown>;
}
