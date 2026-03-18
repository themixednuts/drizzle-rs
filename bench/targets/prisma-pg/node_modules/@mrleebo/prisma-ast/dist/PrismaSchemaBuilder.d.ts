import * as schema from './getSchema';
import { PrintOptions } from './printSchema';
import * as finder from './finder';
type ReplaceReturnType<Original extends (...args: any) => any, NewReturn> = (...a: Parameters<Original>) => NewReturn;
type ExtractKeys = 'getSchema' | 'getSubject' | 'getParent' | 'print';
type NeutralKeys = 'break' | 'comment' | 'attribute' | 'enumerator' | 'then' | 'findByType' | 'findAllByType';
type DatasourceOrGeneratorKeys = 'assignment';
type EnumKeys = 'enumerator';
type FieldKeys = 'attribute' | 'removeAttribute';
type BlockKeys = 'blockAttribute' | 'field' | 'removeField';
type PrismaSchemaFinderOptions = finder.ByTypeOptions & {
    within?: finder.ByTypeSourceObject[];
};
type PrismaSchemaSubset<Universe extends keyof ConcretePrismaSchemaBuilder, Method> = ReplaceReturnType<ConcretePrismaSchemaBuilder[Universe], PrismaSchemaBuilder<Exclude<keyof ConcretePrismaSchemaBuilder, Method>>>;
type PrismaSchemaBuilder<K extends keyof ConcretePrismaSchemaBuilder> = {
    [U in K]: U extends ExtractKeys ? ConcretePrismaSchemaBuilder[U] : U extends NeutralKeys ? ConcretePrismaSchemaBuilder[U] : U extends 'datasource' ? PrismaSchemaSubset<U, 'datasource' | EnumKeys | FieldKeys | BlockKeys> : U extends 'generator' ? PrismaSchemaSubset<U, EnumKeys | FieldKeys | BlockKeys> : U extends 'model' ? PrismaSchemaSubset<U, DatasourceOrGeneratorKeys | EnumKeys | FieldKeys> : U extends 'view' ? PrismaSchemaSubset<U, DatasourceOrGeneratorKeys | EnumKeys | FieldKeys> : U extends 'type' ? PrismaSchemaSubset<U, DatasourceOrGeneratorKeys | EnumKeys | FieldKeys> : U extends 'field' ? PrismaSchemaSubset<U, DatasourceOrGeneratorKeys | EnumKeys> : U extends 'removeField' ? PrismaSchemaSubset<U, DatasourceOrGeneratorKeys | EnumKeys | FieldKeys> : U extends 'enum' ? PrismaSchemaSubset<U, DatasourceOrGeneratorKeys | BlockKeys | FieldKeys> : U extends 'removeAttribute' ? PrismaSchemaSubset<U, DatasourceOrGeneratorKeys | EnumKeys> : PrismaSchemaSubset<U, DatasourceOrGeneratorKeys | EnumKeys | FieldKeys | BlockKeys | 'comment'>;
};
type Arg = string | {
    name: string;
    function?: Arg[];
};
type Subject = schema.Block | schema.Field | schema.Enumerator | undefined;
export declare class ConcretePrismaSchemaBuilder {
    private schema;
    private _subject;
    private _parent;
    constructor(source?: string);
    print(options?: PrintOptions): string;
    getSchema(): schema.Schema;
    generator(name: string, provider?: string): this;
    drop(name: string): this;
    datasource(provider: string, url: string | {
        env: string;
    }): this;
    model(name: string): this;
    view(name: string): this;
    type(name: string): this;
    enum(name: string, enumeratorNames?: string[]): this;
    enumerator(value: string): this;
    private getSubject;
    private getParent;
    blockAttribute(name: string, args?: string | string[] | Record<string, schema.Value>): this;
    attribute<T extends schema.Field>(name: string, args?: Arg[] | Record<string, string[]>): this;
    removeAttribute<T extends schema.Field>(name: string): this;
    assignment<T extends schema.Generator | schema.Datasource>(key: string, value: string): this;
    findByType<const Match extends finder.ByTypeMatch>(typeToMatch: Match, { within, ...options }: PrismaSchemaFinderOptions): finder.FindByBlock<Match> | null;
    findAllByType<const Match extends finder.ByTypeMatch>(typeToMatch: Match, { within, ...options }: PrismaSchemaFinderOptions): Array<finder.FindByBlock<Match> | null>;
    private blockInsert;
    break(): this;
    comment(text: string, node?: boolean): this;
    schemaComment(text: string, node?: boolean): this;
    field(name: string, fieldType?: string | schema.Func): this;
    removeField(name: string): this;
    then<R extends NonNullable<Subject>>(callback: (subject: R) => unknown): this;
}
export declare function createPrismaSchemaBuilder(source?: string): PrismaSchemaBuilder<Exclude<keyof ConcretePrismaSchemaBuilder, DatasourceOrGeneratorKeys | EnumKeys | FieldKeys | BlockKeys>>;
export {};
