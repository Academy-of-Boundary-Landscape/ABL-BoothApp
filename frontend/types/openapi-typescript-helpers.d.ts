// openapi-typescript-helpers@0.1.0 的类型定义，本地覆盖版（通过 tsconfig 的 paths 生效，只影响类型检查）。
//
// 唯一改动：Readable / Writable 开头加了「原始类型原样返回」一支。
// 原版对 `T extends object` 逐键映射，而 branded 金额 `Cents = number & { __brand }` 在 TS 看来
// 也是 object，于是被映射成 `{ toFixed: {}; … }`，丢掉了 number——所有带金额的响应都赋不回
// schema 类型。我们的 schema 没有 readOnly/writeOnly 标记，这两个类型本应是恒等映射。
//
// 升级 openapi-fetch / openapi-typescript-helpers 时：src/api/core.spec.ts 有一条测试钉住
// 已安装版本；版本变了就重新从 node_modules 复制一份再打同样的补丁（或确认上游已修）。

type HttpMethod = 'get' | 'put' | 'post' | 'delete' | 'options' | 'head' | 'patch' | 'trace'
type OkStatus = 200 | 201 | 202 | 203 | 204 | 206 | 207 | '2XX'
type ErrorStatus =
  | 500
  | 501
  | 502
  | 503
  | 504
  | 505
  | 506
  | 507
  | 508
  | 510
  | 511
  | '5XX'
  | 400
  | 401
  | 402
  | 403
  | 404
  | 405
  | 406
  | 407
  | 408
  | 409
  | 410
  | 411
  | 412
  | 413
  | 414
  | 415
  | 416
  | 417
  | 418
  | 420
  | 421
  | 422
  | 423
  | 424
  | 425
  | 426
  | 427
  | 428
  | 429
  | 430
  | 431
  | 444
  | 450
  | 451
  | 497
  | 498
  | 499
  | '4XX'
  | 'default'
type OKStatusUnion<T> = FilterKeys<T, OkStatus>
type FirstErrorStatus<T> = T extends {
  500: any
}
  ? T[500]
  : T extends {
        501: any
      }
    ? T[501]
    : T extends {
          502: any
        }
      ? T[502]
      : T extends {
            503: any
          }
        ? T[503]
        : T extends {
              504: any
            }
          ? T[504]
          : T extends {
                505: any
              }
            ? T[505]
            : T extends {
                  506: any
                }
              ? T[506]
              : T extends {
                    507: any
                  }
                ? T[507]
                : T extends {
                      508: any
                    }
                  ? T[508]
                  : T extends {
                        510: any
                      }
                    ? T[510]
                    : T extends {
                          511: any
                        }
                      ? T[511]
                      : T extends {
                            '5XX': any
                          }
                        ? T['5XX']
                        : T extends {
                              400: any
                            }
                          ? T[400]
                          : T extends {
                                401: any
                              }
                            ? T[401]
                            : T extends {
                                  402: any
                                }
                              ? T[402]
                              : T extends {
                                    403: any
                                  }
                                ? T[403]
                                : T extends {
                                      404: any
                                    }
                                  ? T[404]
                                  : T extends {
                                        405: any
                                      }
                                    ? T[405]
                                    : T extends {
                                          406: any
                                        }
                                      ? T[406]
                                      : T extends {
                                            407: any
                                          }
                                        ? T[407]
                                        : T extends {
                                              408: any
                                            }
                                          ? T[408]
                                          : T extends {
                                                409: any
                                              }
                                            ? T[409]
                                            : T extends {
                                                  410: any
                                                }
                                              ? T[410]
                                              : T extends {
                                                    411: any
                                                  }
                                                ? T[411]
                                                : T extends {
                                                      412: any
                                                    }
                                                  ? T[412]
                                                  : T extends {
                                                        413: any
                                                      }
                                                    ? T[413]
                                                    : T extends {
                                                          414: any
                                                        }
                                                      ? T[414]
                                                      : T extends {
                                                            415: any
                                                          }
                                                        ? T[415]
                                                        : T extends {
                                                              416: any
                                                            }
                                                          ? T[416]
                                                          : T extends {
                                                                417: any
                                                              }
                                                            ? T[417]
                                                            : T extends {
                                                                  418: any
                                                                }
                                                              ? T[418]
                                                              : T extends {
                                                                    420: any
                                                                  }
                                                                ? T[420]
                                                                : T extends {
                                                                      421: any
                                                                    }
                                                                  ? T[421]
                                                                  : T extends {
                                                                        422: any
                                                                      }
                                                                    ? T[422]
                                                                    : T extends {
                                                                          423: any
                                                                        }
                                                                      ? T[423]
                                                                      : T extends {
                                                                            424: any
                                                                          }
                                                                        ? T[424]
                                                                        : T extends {
                                                                              425: any
                                                                            }
                                                                          ? T[425]
                                                                          : T extends {
                                                                                426: any
                                                                              }
                                                                            ? T[426]
                                                                            : T extends {
                                                                                  427: any
                                                                                }
                                                                              ? T[427]
                                                                              : T extends {
                                                                                    428: any
                                                                                  }
                                                                                ? T[428]
                                                                                : T extends {
                                                                                      429: any
                                                                                    }
                                                                                  ? T[429]
                                                                                  : T extends {
                                                                                        430: any
                                                                                      }
                                                                                    ? T[430]
                                                                                    : T extends {
                                                                                          431: any
                                                                                        }
                                                                                      ? T[431]
                                                                                      : T extends {
                                                                                            444: any
                                                                                          }
                                                                                        ? T[444]
                                                                                        : T extends {
                                                                                              450: any
                                                                                            }
                                                                                          ? T[450]
                                                                                          : T extends {
                                                                                                451: any
                                                                                              }
                                                                                            ? T[451]
                                                                                            : T extends {
                                                                                                  497: any
                                                                                                }
                                                                                              ? T[497]
                                                                                              : T extends {
                                                                                                    498: any
                                                                                                  }
                                                                                                ? T[498]
                                                                                                : T extends {
                                                                                                      499: any
                                                                                                    }
                                                                                                  ? T[499]
                                                                                                  : T extends {
                                                                                                        '4XX': any
                                                                                                      }
                                                                                                    ? T['4XX']
                                                                                                    : T extends {
                                                                                                          default: any
                                                                                                        }
                                                                                                      ? T['default']
                                                                                                      : never
type PathsWithMethod<Paths extends {}, PathnameMethod extends HttpMethod> = {
  [Pathname in keyof Paths]: Paths[Pathname] extends {
    [K in PathnameMethod]: any
  }
    ? Pathname
    : never
}[keyof Paths]
interface OperationObject {
  parameters: any
  requestBody: any
  responses: any
}
type PathItemObject = {
  [M in HttpMethod]: OperationObject
} & {
  parameters?: any
}
type ResponseObjectMap<T> = T extends {
  responses: any
}
  ? T['responses']
  : unknown
type ResponseContent<T> = T extends {
  content: any
}
  ? T['content']
  : unknown
type OperationRequestBody<T> = 'requestBody' extends keyof T ? T['requestBody'] : never
type PickRequestBody<T> = 'requestBody' extends keyof T ? Pick<T, 'requestBody'> : never
type IsOperationRequestBodyOptional<T> =
  RequiredKeysOf<PickRequestBody<T>> extends never ? true : false
type OperationRequestBodyMediaContent<T> =
  IsOperationRequestBodyOptional<T> extends true
    ? ResponseContent<NonNullable<OperationRequestBody<T>>> | undefined
    : ResponseContent<OperationRequestBody<T>>
type OperationRequestBodyContent<T> =
  FilterKeys<OperationRequestBodyMediaContent<T>, MediaType> extends never
    ? FilterKeys<NonNullable<OperationRequestBodyMediaContent<T>>, MediaType> | undefined
    : FilterKeys<OperationRequestBodyMediaContent<T>, MediaType>
type SuccessResponse<
  T extends Record<string | number, any>,
  Media extends MediaType = MediaType,
> = GetResponseContent<T, Media, OkStatus>
type GetResponseContent<
  T extends Record<string | number, any>,
  Media extends MediaType = MediaType,
  ResponseCode extends keyof T = keyof T,
> = ResponseCode extends keyof T
  ? {
      [K in ResponseCode]: T[K]['content'] extends Record<string, any>
        ? FilterKeys<T[K]['content'], Media> extends never
          ? T[K]['content']
          : FilterKeys<T[K]['content'], Media>
        : K extends keyof T
          ? T[K]['content']
          : never
    }[ResponseCode]
  : never
type ErrorResponse<
  T extends Record<string | number, any>,
  Media extends MediaType = MediaType,
> = GetResponseContent<T, Media, ErrorStatus>
type SuccessResponseJSON<PathMethod extends Record<string | number, any>> = SuccessResponse<
  ResponseObjectMap<PathMethod>,
  `${string}/json`
>
type ErrorResponseJSON<PathMethod extends Record<string | number, any>> = ErrorResponse<
  ResponseObjectMap<PathMethod>,
  `${string}/json`
>
type RequestBodyJSON<PathMethod> = JSONLike<FilterKeys<OperationRequestBody<PathMethod>, 'content'>>
type FilterKeys<Obj, Matchers> = Obj[keyof Obj & Matchers]
type GetValueWithDefault<Obj, KeyPattern, Default> = Obj extends any
  ? FilterKeys<Obj, KeyPattern> extends never
    ? Default
    : FilterKeys<Obj, KeyPattern>
  : never
type MediaType = `${string}/${string}`
type JSONLike<T> = FilterKeys<T, `${string}/json`>
type FindRequiredKeys<T, K extends keyof T> = K extends unknown
  ? undefined extends T[K]
    ? never
    : K
  : K
type HasRequiredKeys<T> = FindRequiredKeys<T, keyof T>
type RequiredKeysOfHelper<T> = {
  [K in keyof T]: {} extends Pick<T, K> ? never : K
}[keyof T]
type RequiredKeysOf<T> = RequiredKeysOfHelper<T> extends undefined ? never : RequiredKeysOfHelper<T>
type $Read<T> = {
  readonly $read: T
}
type $Write<T> = {
  readonly $write: T
}
type Readable<T> = T extends string | number | boolean | bigint | symbol | null | undefined
  ? T
  : T extends $Write<any>
    ? never
    : T extends $Read<infer U>
      ? Readable<U>
      : T extends (infer E)[]
        ? Readable<E>[]
        : T extends object
          ? {
              [K in keyof T as NonNullable<T[K]> extends $Write<any> ? never : K]: Readable<T[K]>
            }
          : T
type Writable<T> = T extends string | number | boolean | bigint | symbol | null | undefined
  ? T
  : T extends $Read<any>
    ? never
    : T extends $Write<infer U>
      ? Writable<U>
      : T extends (infer E)[]
        ? Writable<E>[]
        : T extends object
          ? {
              [K in keyof T as NonNullable<T[K]> extends $Read<any> ? never : K]: Writable<T[K]>
            } & {
              [K in keyof T as NonNullable<T[K]> extends $Read<any> ? K : never]?: never
            }
          : T

export type {
  $Read,
  $Write,
  ErrorResponse,
  ErrorResponseJSON,
  ErrorStatus,
  FilterKeys,
  FindRequiredKeys,
  FirstErrorStatus,
  GetResponseContent,
  GetValueWithDefault,
  HasRequiredKeys,
  HttpMethod,
  IsOperationRequestBodyOptional,
  JSONLike,
  MediaType,
  OKStatusUnion,
  OkStatus,
  OperationObject,
  OperationRequestBody,
  OperationRequestBodyContent,
  OperationRequestBodyMediaContent,
  PathItemObject,
  PathsWithMethod,
  Readable,
  RequestBodyJSON,
  RequiredKeysOf,
  ResponseContent,
  ResponseObjectMap,
  SuccessResponse,
  SuccessResponseJSON,
  Writable,
}
