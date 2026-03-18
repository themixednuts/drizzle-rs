import { PrismaParser } from './parser';
import { ICstVisitor } from 'chevrotain';
type Class<T> = new (...args: any[]) => T;
export type PrismaVisitor = ICstVisitor<any, any>;
export declare const VisitorClassFactory: (parser: PrismaParser) => Class<PrismaVisitor>;
export declare const DefaultVisitorClass: Class<PrismaVisitor>;
export declare const defaultVisitor: PrismaVisitor;
export {};
