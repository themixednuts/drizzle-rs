import type { CstNode, IToken } from 'chevrotain';
import * as schema from './getSchema';
declare const schemaObjects: readonly ["model", "view", "type"];
export declare function isOneOfSchemaObjects<T extends string>(obj: schema.Object, schemas: readonly T[]): obj is Extract<schema.Object, {
    type: T;
}>;
export declare function isSchemaObject(obj: schema.Object): obj is Extract<schema.Object, {
    type: (typeof schemaObjects)[number];
}>;
declare const fieldObjects: readonly ["field", "enumerator"];
export declare function isSchemaField(field: schema.Field | schema.Enumerator): field is Extract<schema.Field, {
    type: (typeof fieldObjects)[number];
}>;
export declare function isToken(node: [IToken] | [CstNode]): node is [IToken];
export declare function appendLocationData<T extends Record<string, unknown>>(data: T, ...tokens: IToken[]): T;
export {};
