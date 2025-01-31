export enum NodeEventKind {
  Destory = 'Destory',
}

export interface NodeEvent {
  kind: NodeEventKind;
  targetId: number;
}
