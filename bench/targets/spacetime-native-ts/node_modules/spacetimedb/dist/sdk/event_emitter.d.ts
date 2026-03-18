export declare class EventEmitter<Key, Callback extends Function = Function> {
    #private;
    on(event: Key, callback: Callback): void;
    off(event: Key, callback: Callback): void;
    emit(event: Key, ...args: any[]): void;
}
//# sourceMappingURL=event_emitter.d.ts.map