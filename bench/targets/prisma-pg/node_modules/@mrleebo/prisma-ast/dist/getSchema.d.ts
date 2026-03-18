import { PrismaVisitor } from './visitor';
import type { CstNodeLocation } from 'chevrotain';
import { PrismaParser } from './parser';
export declare function getSchema(source: string, options?: {
    parser: PrismaParser;
    visitor: PrismaVisitor;
}): Schema;
export interface Schema {
    type: 'schema';
    list: Block[];
}
export type Block = Model | View | Datasource | Generator | Enum | Comment | Break | Type;
export interface Object {
    type: 'model' | 'view' | 'type';
    name: string;
    properties: Array<Property | Comment | Break>;
}
export interface Model extends Object {
    type: 'model';
    location?: CstNodeLocation;
}
export interface View extends Object {
    type: 'view';
    location?: CstNodeLocation;
}
export interface Type extends Object {
    type: 'type';
    location?: CstNodeLocation;
}
export interface Datasource {
    type: 'datasource';
    name: string;
    assignments: Array<Assignment | Comment | Break>;
    location?: CstNodeLocation;
}
export interface Generator {
    type: 'generator';
    name: string;
    assignments: Array<Assignment | Comment | Break>;
    location?: CstNodeLocation;
}
export interface Enum {
    type: 'enum';
    name: string;
    enumerators: Array<Enumerator | Comment | Break | BlockAttribute | GroupedAttribute>;
    location?: CstNodeLocation;
}
export interface Comment {
    type: 'comment';
    text: string;
}
export interface Break {
    type: 'break';
}
export type Property = GroupedBlockAttribute | BlockAttribute | Field;
export interface Assignment {
    type: 'assignment';
    key: string;
    value: Value;
}
export interface Enumerator {
    type: 'enumerator';
    name: string;
    value?: Value;
    attributes?: Attribute[];
    comment?: string;
}
export interface BlockAttribute {
    type: 'attribute';
    kind: 'object' | 'view' | 'type';
    group?: string;
    name: string;
    args: AttributeArgument[];
    location?: CstNodeLocation;
}
export type GroupedBlockAttribute = BlockAttribute & {
    group: string;
};
export interface Field {
    type: 'field';
    name: string;
    fieldType: string | Func;
    array?: boolean;
    optional?: boolean;
    attributes?: Attribute[];
    comment?: string;
    location?: CstNodeLocation;
}
export type Attr = Attribute | GroupedAttribute | BlockAttribute | GroupedBlockAttribute;
export interface Attribute {
    type: 'attribute';
    kind: 'field';
    group?: string;
    name: string;
    args?: AttributeArgument[];
    location?: CstNodeLocation;
}
export type GroupedAttribute = Attribute & {
    group: string;
};
export interface AttributeArgument {
    type: 'attributeArgument';
    value: KeyValue | Value | Func;
}
export interface KeyValue {
    type: 'keyValue';
    key: string;
    value: Value;
}
export interface Func {
    type: 'function';
    name: string;
    params?: Value[];
}
export interface RelationArray {
    type: 'array';
    args: string[];
}
export type Value = string | number | boolean | Func | RelationArray | Array<Value>;
