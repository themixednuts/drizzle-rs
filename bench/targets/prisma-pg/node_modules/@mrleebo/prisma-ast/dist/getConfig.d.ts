import type { IParserConfig } from 'chevrotain';
export type PrismaAstParserConfig = Pick<IParserConfig, 'nodeLocationTracking'>;
export interface PrismaAstConfig {
    parser: PrismaAstParserConfig;
}
export default function getConfig(): PrismaAstConfig;
