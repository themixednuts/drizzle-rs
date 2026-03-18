import * as Types from './getSchema';
type Block = 'generator' | 'datasource' | 'model' | 'view' | 'enum' | 'type';
export interface PrintOptions {
    sort?: boolean;
    locales?: string | string[];
    sortOrder?: Block[];
}
export declare function printSchema(schema: Types.Schema, options?: PrintOptions): string;
export {};
