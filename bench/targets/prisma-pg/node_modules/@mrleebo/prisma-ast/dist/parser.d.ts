import { CstParser } from 'chevrotain';
import { PrismaAstParserConfig } from './getConfig';
export declare class PrismaParser extends CstParser {
    readonly config: PrismaAstParserConfig;
    constructor(config: PrismaAstParserConfig);
    private break;
    private keyedArg;
    private array;
    private func;
    private value;
    private property;
    private assignment;
    private field;
    private block;
    private enum;
    private fieldAttribute;
    private blockAttribute;
    private attributeArg;
    private component;
    private comment;
    schema: import("chevrotain").ParserMethod<[], import("chevrotain").CstNode>;
}
export declare const defaultParser: PrismaParser;
