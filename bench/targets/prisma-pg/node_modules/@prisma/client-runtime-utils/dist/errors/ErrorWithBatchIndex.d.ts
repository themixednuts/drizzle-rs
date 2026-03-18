export interface ErrorWithBatchIndex {
    batchRequestIdx?: number;
}
export declare function hasBatchIndex(value: object): value is Required<ErrorWithBatchIndex>;
