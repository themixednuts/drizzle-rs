import type * as schema from './getSchema';
export type ByTypeSourceObject = schema.Block | schema.Enumerator | schema.Field | schema.Property | schema.Attribute | schema.Assignment;
export type ByTypeMatchObject = Exclude<ByTypeSourceObject, schema.Comment | schema.Break>;
export type ByTypeMatch = ByTypeMatchObject['type'];
export type ByTypeOptions = {
    name?: string | RegExp;
};
export type FindByBlock<Match> = Extract<ByTypeMatchObject, {
    type: Match;
}>;
export declare const findByType: <const Match extends "model" | "view" | "datasource" | "generator" | "enum" | "type" | "enumerator" | "field" | "attribute" | "assignment">(list: ByTypeSourceObject[], typeToMatch: Match, options?: ByTypeOptions) => FindByBlock<Match> | null;
export declare const findAllByType: <const Match extends "model" | "view" | "datasource" | "generator" | "enum" | "type" | "enumerator" | "field" | "attribute" | "assignment">(list: ByTypeSourceObject[], typeToMatch: Match, options?: ByTypeOptions) => FindByBlock<Match>[];
