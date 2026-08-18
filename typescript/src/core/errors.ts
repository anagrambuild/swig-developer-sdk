export class SwigDeveloperSdkError extends Error {
  constructor(
    message: string,
    public readonly code: string,
    public readonly statusCode: number,
    public readonly details?: unknown,
  ) {
    super(message);
    this.name = 'SwigDeveloperSdkError';
  }

  static fromResponse(response: Response, body?: unknown) {
    const errorBody = isRecord(body) ? body : undefined;
    const nestedError = isRecord(errorBody?.error)
      ? errorBody.error
      : undefined;

    if (nestedError) {
      return new SwigDeveloperSdkError(
        optionalString(nestedError.message) ??
          `Request failed with status ${response.status}`,
        optionalString(nestedError.code) ?? `HTTP_${response.status}`,
        response.status,
        nestedError.details ?? errorBody,
      );
    }

    return new SwigDeveloperSdkError(
      optionalString(errorBody?.message) ??
        `Request failed with status ${response.status}`,
      optionalString(errorBody?.code) ?? `HTTP_${response.status}`,
      response.status,
      errorBody?.details ?? body,
    );
  }

  static fromException(error: unknown) {
    if (error instanceof SwigDeveloperSdkError) {
      return error;
    }

    if (error instanceof Error) {
      return new SwigDeveloperSdkError(error.message, 'NETWORK_ERROR', 0);
    }

    return new SwigDeveloperSdkError(
      'An unknown error occurred',
      'UNKNOWN_ERROR',
      0,
    );
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function optionalString(value: unknown): string | undefined {
  return typeof value === 'string' ? value : undefined;
}
