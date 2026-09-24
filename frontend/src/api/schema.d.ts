/* eslint-disable */
// 由 frontend/scripts/gen-api.mjs 从 src-tauri/openapi.json 生成。
// 不要手改——改后端，UPDATE_OPENAPI=1 跑 cargo test，再 npm run gen:api。
import type { Cents } from '@/utils/money'

export interface paths {
    "/channels": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * 已用过的收款渠道，跨展会。
         * @description 挂在 `/api/channels` 而不是 `/api/events/:id/channels`：它按定义就是跨展会的。
         *     这是防「微信」和「微信支付」分裂成两个账户的那一条（②-1/②-2 交接段第 3 条）。
         */
        get: operations["list_channels"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/events/{event_id}/adjustments": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 这场展会的结算调整列表。金额带符号：负 = 我要多给他们。 */
        get: operations["list_adjustments"];
        put?: never;
        /**
         * 登记一笔结算调整。方向由 `direction` 表达，金额恒为正。
         * @description 展会已结算后仍可调用（冻结例外：垫付、结算调整、收摊清点三类）。
         */
        post: operations["create_adjustment"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/events/{event_id}/adjustments/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /**
         * 删除一笔结算调整——写一条冲正 journal，原记录保留。
         * @description 展会已结算后仍可调用（冻结例外：垫付、结算调整、收摊清点三类）。
         */
        delete: operations["delete_adjustment"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/events/{event_id}/advances": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 这场展会的垫付列表。 */
        get: operations["list_advances"];
        put?: never;
        /**
         * 登记一笔垫付（摊主替货主掏的钱）。
         * @description 展会已结算后仍可调用（冻结例外：垫付、结算调整、收摊清点三类）。
         */
        post: operations["create_advance"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/events/{event_id}/advances/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /**
         * 删除一笔垫付——写一条冲正 journal，原记录保留。
         * @description 展会已结算后仍可调用（冻结例外：垫付、结算调整、收摊清点三类）。
         */
        delete: operations["delete_advance"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/events/{event_id}/settlement": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 结算单。页面与 xlsx 导出渲染的是同一个 `SettlementReport`。 */
        get: operations["get_settlement"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/events/{event_id}/settlement.xlsx": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 结算单导出为四 sheet 的 xlsx。 */
        get: operations["download_settlement_xlsx"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/events/{event_id}/settlement/reconcile": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * 收摊清点：按渠道填实际到手，差额记到摊主名下。
         * @description 展会已结算后仍可调用（冻结例外：垫付、结算调整、收摊清点三类）。必须覆盖这场展会用过的每个渠道，且每个渠道只出现一次。
         */
        post: operations["reconcile"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
}
export type webhooks = Record<string, never>;
export interface components {
    schemas: {
        AdjustmentRequest: {
            /** @description 分，必须为正。符号由 `direction` 决定。 */
            amount: components["schemas"]["Money"];
            /**
             * @description `to_them` = 我要多给他们；`to_me` = 他们要多给我。
             *
             *     **界面不给摊主填正负号。** 母 spec 5.2 那个例子自己都要算一遍才对得上
             *     方向，让人在收摊后的疲惫状态下判断「赔付该填正还是负」是设计失误。
             */
            direction: string;
            label: string;
            /** Format: int64 */
            society_id: number;
        };
        AdvanceRequest: {
            /** @description 分，必须为正。方向是固定的（摊主掏钱给社团），不需要符号。 */
            amount: components["schemas"]["Money"];
            label: string;
            /** Format: int64 */
            society_id: number;
        };
        /** @description 所有错误响应的形状。为什么必须是这个形状，见 `crate::error` 的模块注释。 */
        ApiErrorBody: {
            error: string;
        };
        ChannelActual: {
            /** @description 摊主数出来的实际到手（分）。 */
            actual: components["schemas"]["Money"];
            channel: string;
        };
        ChannelLine: {
            actual: components["schemas"]["Money"];
            book: components["schemas"]["Money"];
            channel: string;
            counted: boolean;
            diff: components["schemas"]["Money"];
        };
        CreatedEntry: {
            /** Format: int64 */
            id: number;
            /** Format: int64 */
            journal_id: number;
        };
        /**
         * @description 一个商品在一场展会里的全部去向。**每一项都从 `stock_movements` 按方向取**，
         *     没有任何缓存字段——母 spec 的整个设计就是「不存在第二个可以漂移的数字」。
         */
        GoodsLine: {
            /** @description 货主应得（已扣除退货部分）。**用 allocated 不用 paid。** */
            allocated: components["schemas"]["Money"];
            /**
             * Format: int64
             * @description `外部 → 现场仓` 的合计。开场带货和中途补货是同一件事。
             */
            brought_in: number;
            /** Format: int64 */
            event_product_id: number;
            /** Format: int64 */
            gifted: number;
            /**
             * @description 这个商品的原价合计（已扣除退货部分）。spec 8.4 要求「货主明细」按商品列出金额——
             *     只给数量等于让社团自己去乘单价，而 Lot 折让之后单价不等于成交价。
             */
            gross: components["schemas"]["Money"];
            /** @description Lot 折让 = gross − allocated。 */
            lot_discount: components["schemas"]["Money"];
            name: string;
            /**
             * Format: int64
             * @description 还在现场仓的。走完带回之后必须是 0。
             */
            on_site: number;
            product_code: string;
            /**
             * Format: int64
             * @description `损耗` 的净余额，含「退货到损耗」的那部分。
             */
            scrapped: number;
            /**
             * Format: int64
             * @description `顾客仓` 的净余额。退货会自动减回去。
             */
            sold: number;
            /**
             * Format: int64
             * @description `现场仓 → 外部` 的合计。
             */
            taken_back: number;
            /**
             * Format: int64
             * @description `差异` 的净余额。盘亏为正、盘盈为负。
             */
            variance: number;
        };
        GoodsTotals: {
            /** Format: int64 */
            brought_in: number;
            /** Format: int64 */
            gifted: number;
            /** Format: int64 */
            on_site: number;
            /** Format: int64 */
            scrapped: number;
            /** Format: int64 */
            sold: number;
            /** Format: int64 */
            taken_back: number;
            /** Format: int64 */
            variance: number;
        };
        LedgerEntryRow: {
            /** @description 垫付恒为正；结算调整带符号（负 = 我要多给他们）。 */
            amount: components["schemas"]["Money"];
            /** Format: int64 */
            id: number;
            label: string;
            /** Format: int64 */
            owner_society_id: number;
            society_name: string;
        };
        /**
         * Format: cents
         * @description 金额，单位：分
         */
        Money: Cents;
        ReconcileRequest: {
            counts: components["schemas"]["ChannelActual"][];
        };
        ReconcileResponse: {
            /**
             * Format: int64
             * @description 分文不差时为 `None`——没有腿可记，「数过了」靠 `events.reconciled_at`。
             */
            journal_id?: number | null;
        };
        SettlementEntry: {
            /**
             * @description 按对「我应转给」的影响存：垫付为正（算出来要减），
             *     结算调整正数 = 我要多给他们（算出来要加）。
             */
            amount: components["schemas"]["Money"];
            /**
             * @description 结算调整取 `settlement_adjustments.created_at`；垫付恒为 `None`
             *     （`advances` 表没有这一列，迁移已冻结）。
             *     母 spec §7 的样例里调整那一行是带日期的：「10-03 追加：…」。
             */
            at?: string | null;
            label: string;
        };
        SettlementReport: {
            actual_total: components["schemas"]["Money"];
            channels: components["schemas"]["ChannelLine"][];
            event_date: string;
            event_name: string;
            generated_at: string;
            last_changed_at?: string | null;
            societies: components["schemas"]["SocietyBlock"][];
            stocktaken: boolean;
            stocktaken_at?: string | null;
            transfer_total: components["schemas"]["Money"];
            /** @description 摊主留存 = 实际到手合计 − Σ 我应转给各货主。 */
            vendor_retained: components["schemas"]["Money"];
            /** @description 恒等式没撞上的地方。**页面和导出都要显眼地显示它**。 */
            warnings: string[];
        };
        SocietyBlock: {
            adjustments: components["schemas"]["SettlementEntry"][];
            adjustments_total: components["schemas"]["Money"];
            advances: components["schemas"]["SettlementEntry"][];
            advances_total: components["schemas"]["Money"];
            gift_self_paid: components["schemas"]["Money"];
            goods: components["schemas"]["GoodsLine"][];
            gross: components["schemas"]["Money"];
            is_home: boolean;
            lot_discount: components["schemas"]["Money"];
            manual_discount: components["schemas"]["Money"];
            name: string;
            net: components["schemas"]["Money"];
            refund_kept: components["schemas"]["Money"];
            /** Format: int64 */
            society_id: number;
            totals: components["schemas"]["GoodsTotals"];
            /** @description 我应转给他们 = −（往来余额）。 */
            transfer: components["schemas"]["Money"];
        };
    };
    responses: never;
    parameters: never;
    requestBodies: never;
    headers: never;
    pathItems: never;
}
export type $defs = Record<string, never>;
export interface operations {
    list_channels: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 预置渠道在前，历史用过的在后，去重 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": string[];
                };
            };
            /** @description 未登录或令牌无效 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 无权访问这场展会 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    list_adjustments: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 展会 id */
                event_id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["LedgerEntryRow"][];
                };
            };
            /** @description 未登录或令牌无效 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 无权访问这场展会 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    create_adjustment: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 展会 id */
                event_id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["AdjustmentRequest"];
            };
        };
        responses: {
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CreatedEntry"];
                };
            };
            /** @description 金额非正、方向不认识、说明为空或过长、社团不存在 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 未登录或令牌无效 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 无权访问这场展会 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 展会不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    delete_adjustment: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 展会 id */
                event_id: number;
                /** @description 条目 id */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 已冲正 */
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未登录或令牌无效 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 无权访问这场展会 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 条目不存在或不属于这场展会 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 已经删过了 */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    list_advances: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 展会 id */
                event_id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["LedgerEntryRow"][];
                };
            };
            /** @description 未登录或令牌无效 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 无权访问这场展会 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    create_advance: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 展会 id */
                event_id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["AdvanceRequest"];
            };
        };
        responses: {
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CreatedEntry"];
                };
            };
            /** @description 金额非正、说明为空或过长、社团不存在 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 未登录或令牌无效 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 无权访问这场展会 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 展会不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    delete_advance: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 展会 id */
                event_id: number;
                /** @description 条目 id */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 已冲正 */
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description 未登录或令牌无效 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 无权访问这场展会 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 条目不存在或不属于这场展会 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 已经删过了 */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    get_settlement: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 展会 id */
                event_id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["SettlementReport"];
                };
            };
            /** @description 未登录或令牌无效 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 无权访问这场展会 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 展会不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    download_settlement_xlsx: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 展会 id */
                event_id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet": number[];
                };
            };
            /** @description 未登录或令牌无效 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 无权访问这场展会 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 展会不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    reconcile: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 展会 id */
                event_id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ReconcileRequest"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ReconcileResponse"];
                };
            };
            /** @description 渠道重复、漏报、或不是这场展会用过的渠道 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 未登录或令牌无效 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 无权访问这场展会 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 展会不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
}
