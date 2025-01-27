export enum NodeEventKind {
  Destory = 'Destory',
  Custom = 'Custom',
}

export interface NodeEvent {
  kind: NodeEventKind;
  targetId: number;
  customKind?: string;
  customBody?: Record<string, unknown>;
}
