import { PrintOptions } from './printSchema';
import { createPrismaSchemaBuilder } from './PrismaSchemaBuilder';
type Options = PrintOptions;
export declare function produceSchema(source: string, producer: (builder: ReturnType<typeof createPrismaSchemaBuilder>) => void, options?: Options): string;
export {};
