import { ErrorWithBatchIndex } from './ErrorWithBatchIndex';
type UnknownErrorParams = {
    clientVersion: string;
    batchRequestIdx?: number;
};
export declare class PrismaClientUnknownRequestError extends Error implements ErrorWithBatchIndex {
    clientVersion: string;
    batchRequestIdx?: number;
    constructor(message: string, { clientVersion, batchRequestIdx }: UnknownErrorParams);
    get [Symbol.toStringTag](): string;
}
export {};
