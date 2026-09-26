/* eslint-disable */
// 由 frontend/scripts/gen-api.mjs 从 src-tauri/openapi.json 生成。
// 不要手改——改后端，UPDATE_OPENAPI=1 跑 cargo test，再 npm run gen:api。
import type { Cents } from '@/utils/money'

export interface paths {
    "/admin/default-passwords": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * 管理后台据此常驻提醒改密码。默认密码写在公开文档里，而 LAN 上的任何设备都能访问登录页，
         *     所以这里只**提醒**、不拦登录（摊位现场临时借设备登录是正常用法）。
         */
        get: operations["admin.default_passwords"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/password": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** 修改管理员登录密码。需要管理员。 */
        put: operations["admin.update_admin_password"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/reset-database": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** 清空全部业务数据、删除上传的图片并把密码恢复为默认值（危险操作）。需要管理员。 */
        put: operations["admin.reset_database_handler"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/vendor-default-password": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** 修改全局默认摊主密码。需要管理员。 */
        put: operations["admin.update_vendor_default_password"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/auth/is-default-admin-password": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 管理员密码是否仍是出厂默认值 `admin123`（登录页据此提示改密码）。 */
        get: operations["auth.is_default_admin_password"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/auth/login": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * 登录：管理员校验全局密码，摊主可校验全局密码或展会专属密码。
         * @description 成功时在 Body 里回 token，同时下发 HttpOnly 的 `access_token_cookie`。
         *     `eventId` 只在摊主用「展会专属密码」登录成功时回填。
         */
        post: operations["auth.login_handler"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/auth/logout": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 退出登录：清掉 HttpOnly 的 `access_token_cookie`。 */
        post: operations["auth.logout_handler"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
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
        get: operations["settlement.list_channels"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/events": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 展会列表，可选按 `status` 过滤。公开接口。 */
        get: operations["event.list_events"];
        put?: never;
        /** 创建展会（multipart 表单，可传多张收款码）。需要管理员。 */
        post: operations["event.create_event"];
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
        get: operations["settlement.list_adjustments"];
        put?: never;
        /**
         * 登记一笔结算调整。方向由 `direction` 表达，金额恒为正。
         * @description 展会已结算后仍可调用（冻结例外：垫付、结算调整、收摊清点三类）。
         */
        post: operations["settlement.create_adjustment"];
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
        delete: operations["settlement.delete_adjustment"];
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
        get: operations["settlement.list_advances"];
        put?: never;
        /**
         * 登记一笔垫付（摊主替货主掏的钱）。
         * @description 展会已结算后仍可调用（冻结例外：垫付、结算调整、收摊清点三类）。
         */
        post: operations["settlement.create_advance"];
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
        delete: operations["settlement.delete_advance"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/events/{event_id}/closing": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * 收摊向导的当前状态：待处理订单、现场仓余量、盘点时间与推进阻断项。
         * @description **能不能进下一步由后端说了算**——`blockers` 非空就挡住。
         */
        get: operations["closing.get_closing"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/events/{event_id}/closing/settle": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * 结束展会：把状态置为「已结算」，账本从此冻结。
         * @description 盘点可跳过；结算单靠 `stocktaken_at` 是否为 null 区分「没盘点」。
         */
        post: operations["closing.settle"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/events/{event_id}/closing/stocktake": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * 提交盘点实数。**收全量**：现场仓余额非 0 的商品必须全部出现，数过一致的也要报。
         * @description 盘亏/盘盈写差异腿，只动货不动钱。零差异时不写 journal，但仍落 `stocktaken_at`。
         */
        post: operations["closing.stocktake"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/events/{event_id}/closing/takeback": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 把现场仓余货全部带回（现场仓 → 外部）。全卖光时是空操作，不写 journal。 */
        post: operations["closing.takeback"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/events/{event_id}/gifts": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 这场展会的赠送列表。已被冲正的原条目和冲正条目都不出现。 */
        get: operations["inventory.list_gifts"];
        put?: never;
        /** 登记一笔赠送。默认由货主自己承担；`vendor_pays = true` 时摊主按原价补给货主。 */
        post: operations["inventory.create_gift"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/events/{event_id}/journals/{journal_id}/reverse": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * 撤销一条赠送 / 报废登记。
         * @description **只接受这两种 kind。** 销售和收款有自己的冲正通道（取消订单，
         *     `reverse_order_journals`），进货和盘点没有撤销语义（记错了就再记一条反向的）。
         *     不设这道闸，这个端点就成了一个能把任何 journal 冲掉的万能口子。
         */
        post: operations["inventory.reverse_entry"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/events/{event_id}/lots": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 列出这场展会的全部套装，含候选已被删空的（它们仍需在管理页可见、可删）。 */
        get: operations["lot.list_lots"];
        put?: never;
        /** 新建一个套装（Lot）：候选商品集合 + 要选几件 + 总价，候选必须同一货主。 */
        post: operations["lot.create_lot"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/events/{event_id}/lots/preview": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * 试算一份还没保存的套装配置：它会被顾客怎么用、你最多让多少、哪里可能配错了。
         * @description **零写入**，走和 `create_lot` 完全相同的校验（同一个 `validate_numbers` /
         *     `validate_candidates` / `validate_candidate_count`），所以「跨货主」「候选不属于
         *     本场展会」「不允许重复但候选数不够」这些错，试算和保存报的是同一个 400——
         *     不会出现「试算说行、保存说不行」。
         *
         *     这个端点同时服务两个消费方：配置页的实时预览，以及将来接 LLM 辅助配置时
         *     「提一个方案、立刻看后果」的那一步。**在它之前，配置的后果对人和对机器都是黑箱**——
         *     摊主只能配完等顾客来薅，这正是 2026-09-24 那个缺陷被发现的方式。
         */
        post: operations["lot.preview_lot"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/events/{event_id}/lots/{lot_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** 整体替换一个套装的配置：候选集整组重写，不是追加。 */
        put: operations["lot.update_lot"];
        post?: never;
        /**
         * 删除一个套装。
         * @description 删除永远放行。
         *
         *     `order_lots.lot_id` 是 `ON DELETE SET NULL`，而名字和价格在下单那一刻就
         *     **快照**进了 `order_lots`——历史订单不受影响。所以这里不需要
         *     「被引用就不给删」那种守卫（`api/product.rs` 删商品时要，因为那边没有快照）。
         */
        delete: operations["lot.delete_lot"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/events/{event_id}/orders": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 本场展会的订单列表，可按 `status` 过滤。需要管理员或这场展会的摊主。 */
        get: operations["order.list_orders"];
        put?: never;
        /** 顾客下单（公开接口，不需要 token）。要求展会处于「进行中」。 */
        post: operations["order.create_order"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/events/{event_id}/orders/{order_id}/refunds": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * 一张订单的退货历史，以及每一行的可退余量。
         * @description 按 `order_lines` 逐行返回——同一个商品可能因套装归属拆成多行，前端不要按
         *     `event_product_id` 去重。订单不属于这场展会时返回 404（而不是空列表）。
         */
        get: operations["refund.list_refunds"];
        put?: never;
        /**
         * 创建一笔退货：逐行冲销货主应得、手工折让与顾客退款，记一条退货 journal。
         * @description 只能退**已完成**的订单；待处理/已取消的订单会被拒绝。展会已结算（冻结）后
         *     `require_event_open` 会挡住退货。
         */
        post: operations["refund.create_refund"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/events/{event_id}/orders/{order_id}/status": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** 完成或取消一张订单。需要管理员或这场展会的摊主。 */
        put: operations["order.update_order_status"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/events/{event_id}/products": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 列出某场展会全部商品，带现场库存与累计进货。公开接口。 */
        get: operations["product.list_event_products"];
        put?: never;
        /** 把一个全局商品选进某场展会并记首批进货。需要管理员或本场摊主。 */
        post: operations["product.add_product_to_event"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/events/{event_id}/products/import": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * 批量把商品上架到某场展会，并可选地从别的展会复制套装。需要管理员或本场摊主。
         * @description **单事务**：任一商品重复上架（409，写明商品名，**不跳过**）、任一候选商品映射不到
         *     （400，写明套装名与商品名）、展会已结算（409），都会让整批回滚。套装映射按
         *     `master_product_id`：本请求刚建的商品与目标展会原本就有的商品都算“在本场”。
         */
        post: operations["product.import_products"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/events/{event_id}/products/{id}/restock": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 给某场展会里的商品补货（外部 → 现场仓）。需要管理员或本场摊主。 */
        post: operations["product.restock_product"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/events/{event_id}/quote": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * 给购物车报价。**公开、无鉴权、零写入。**
         * @description 存在的唯一理由是 D3「顾客端价格必须可解释」：购物车要明写
         *     「已应用：本子任选3本100 −20.00」，而不是默默给个低价。
         *
         *     **不查库存**——报价是定价预览，库存由下单把关（所以这里也不必开事务）。
         *
         *     **报价永远不被信任**：下单时服务端用同一份 `price_cart` 独立重算，顾客最终付的
         *     金额取自**下单响应**而不是这里。两次之间摊主完全可能刚改过 Lot 配置。
         */
        post: operations["lot.quote"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/events/{event_id}/sales_summary": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * 销售趋势图数据：按商品汇总 + 可填充空白时段的时间序列。
         *     支持按商品编号、开始/结束日期筛选，时间粒度 30 或 60 分钟。
         * @description 需要管理员，或本场摊主。
         */
        get: operations["stats.get_sales_summary"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/events/{event_id}/sales_summary/download": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * 导出本场销售记录为 xlsx 文件。展会不存在时不返回 404，文件名回落到
         *     `Event {id}` 后照常出表（现状如此，见 REPORT 的「形状清理」）。
         * @description 需要管理员，或本场摊主。
         */
        get: operations["stats.download_sales_summary"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/events/{event_id}/scraps": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 这场展会的报废列表。已被冲正的原条目和冲正条目都不出现。 */
        get: operations["inventory.list_scraps"];
        put?: never;
        /** 登记一笔报废。报废没有「谁买单」，请求里的 `vendor_pays` 恒被忽略。 */
        post: operations["inventory.create_scrap"];
        delete?: never;
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
        get: operations["settlement.get_settlement"];
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
        get: operations["settlement.download_settlement_xlsx"];
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
        post: operations["settlement.reconcile"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/events/{event_id}/stats": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * 展会仪表盘：总额、订单数、售出件数以及按商品汇总。
         * @description 需要管理员，或本场摊主。
         */
        get: operations["stats.get_event_stats"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/events/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 单个展会详情。公开接口。 */
        get: operations["event.get_event"];
        /** 更新展会（multipart 表单；POST 与 PUT 等价）。需要管理员。 */
        put: operations["event.update_event.put"];
        /** 更新展会（multipart 表单；POST 与 PUT 等价）。需要管理员。 */
        post: operations["event.update_event.post"];
        /** 删除展会，连同它的账一起级联删除。需要管理员。 */
        delete: operations["event.delete_event"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/events/{id}/status": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /**
         * 修改展会状态（筹备 / 进行中）。需要管理员。
         * @description 已结算的展会不能通过这里解冻，也不能直接改成已结算——必须走收摊流程。
         */
        put: operations["event.update_status"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/legacy/export.xlsx": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * 把备份库里的展会 / 订单 / 明细导成三张 sheet 的 xlsx。需要管理员。
         * @description 金额**按老库的元原样写进单元格**，不换算成分——老库那一侧就是元，
         *     中间多一次 ×100 / ÷100 只会引入浮点误差。
         *
         *     错误体是原样的 `text/plain`（不是 `{"error": ...}`），保持既有形状不变。
         */
        get: operations["legacy.export_legacy_xlsx"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/legacy/status": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** v1 备份的状态与行数。需要管理员。 */
        get: operations["legacy.get_legacy_status"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/master-products": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 全局商品库列表。默认只列上架商品，`?all=true` 连停用的一起返回。 */
        get: operations["master_product.list_products"];
        put?: never;
        /** 新建一个全局商品（multipart 表单，`image` 为可选文件字段）。需要管理员。 */
        post: operations["master_product.create_product"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/master-products/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** 修改一个全局商品（multipart 表单；为兼容旧前端同时挂 POST 与 PUT）。需要管理员。 */
        put: operations["master_product.update_product.put"];
        /** 修改一个全局商品（multipart 表单；为兼容旧前端同时挂 POST 与 PUT）。需要管理员。 */
        post: operations["master_product.update_product.post"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/master-products/{id}/images": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 列出某个全局商品的全部识别图。 */
        get: operations["master_product.list_product_images"];
        put?: never;
        /** 给全局商品加一张识别图（multipart，`image` 为必填文件字段）。需要管理员。 */
        post: operations["master_product.add_product_image"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/master-products/{id}/images/{image_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** 修改一张识别图（multipart；为兼容旧前端同时挂 POST 与 PUT）。需要管理员。 */
        put: operations["master_product.update_product_image.put"];
        /** 修改一张识别图（multipart；为兼容旧前端同时挂 POST 与 PUT）。需要管理员。 */
        post: operations["master_product.update_product_image.post"];
        /** 删除一张识别图。需要管理员。 */
        delete: operations["master_product.delete_product_image"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/master-products/{id}/status": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** 上架 / 下架一个全局商品。需要管理员。 */
        put: operations["master_product.update_status"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/products/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** 改某场展会里商品的单价。需要管理员或本场摊主。 */
        put: operations["product.update_product"];
        post?: never;
        /** 从某场展会下架商品。需要管理员或本场摊主。 */
        delete: operations["product.delete_product"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/server-info": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 返回本机 LAN 的 IP、端口与各入口 URL，供连接检测与二维码使用。 */
        get: operations["info.server_info_handler"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/societies": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 社团列表：本社团排第一，其余按名字。选品下拉框里本社团永远在最上面。 */
        get: operations["society.list_societies"];
        put?: never;
        /** 新建一个社团。需要管理员。 */
        post: operations["society.create_society"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/societies/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** 修改社团名，或把它设为本社团（旧的会自动降级）。需要管理员。 */
        put: operations["society.update_society"];
        post?: never;
        /** 删除一个社团。仍被商品 / 垫付 / 结算调整引用的不能删。需要管理员。 */
        delete: operations["society.delete_society"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/sync/export-products": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 导出商品库与识别图片为 `.boothpack`（zip）压缩包。需要管理员。 */
        get: operations["sync.export_products"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/sync/import-products": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 以 `multipart/form-data` 上传 `.boothpack` 并导入商品库（向后兼容的老路径）。需要管理员。 */
        post: operations["sync.import_products"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/sync/import-products-raw": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 以 `application/octet-stream` 原始字节上传 `.boothpack` 并导入（Tauri webview 主用）。需要管理员。 */
        post: operations["sync.import_products_raw"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/vision/models": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 列出注册表中的所有 Vision 模型及其安装/激活状态。 */
        get: operations["vision.list_models"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/vision/models/activate": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 激活一个已安装的 Vision 模型，并触发索引重建。 */
        post: operations["vision.activate_model"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/vision/models/install": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 安装（下载）一个 Vision 模型，返回后台任务 id。 */
        post: operations["vision.install_model"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/vision/models/tasks/{task_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 查询模型安装任务的状态。 */
        get: operations["vision.get_install_task"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/vision/models/{model_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /** 删除一个已安装且非当前激活的 Vision 模型文件。 */
        delete: operations["vision.delete_model"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/vision/rebuild": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** 触发 Vision 索引重建；请求体可选，`force_full` 决定全量还是增量。 */
        post: operations["vision.rebuild_index"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/vision/search": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * 以图搜图：multipart 上传查询图片，返回按相似度排序的商品候选。
         * @description 索引未就绪或正在重建时返回 503；`mode` 为 `order`/`admin_event` 时 `event_id` 必填。
         */
        post: operations["vision.search_by_image"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/vision/settings/ep": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 读取 Vision 推理设备（execution provider）配置与可用 GPU 列表。 */
        get: operations["vision.get_ep_setting"];
        /** 设置 Vision 推理设备并立即重新加载模型。 */
        put: operations["vision.set_ep_setting"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/vision/status": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Vision 运行时状态：模型、索引版本/大小、重建进度与当前推理设备。 */
        get: operations["vision.get_vision_status"];
        put?: never;
        post?: never;
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
        /** @description 仅用于 OpenAPI 文档：`add_product_image` 的 multipart 表单字段。 */
        AddProductImageForm: {
            /** Format: binary */
            image?: string | null;
            kind?: string | null;
        };
        /**
         * @description 结算调整方向：`to_them` = 我要多给他们；`to_me` = 他们要多给我。
         * @enum {string}
         */
        AdjustmentDirection: "to_them" | "to_me";
        AdjustmentRequest: {
            /** @description 分，必须为正。符号由 `direction` 决定。 */
            amount: components["schemas"]["Money"];
            /**
             * @description `to_them` = 我要多给他们；`to_me` = 他们要多给我。
             *
             *     **界面不给摊主填正负号。** 母 spec 5.2 那个例子自己都要算一遍才对得上
             *     方向，让人在收摊后的疲惫状态下判断「赔付该填正还是负」是设计失误。
             */
            direction: components["schemas"]["AdjustmentDirection"];
            label: string;
            /** Format: int64 */
            society_id: number;
        };
        /**
         * @description 管理员密码 / 默认摊主密码更新成功的响应。
         *
         *     两个接口共用；`#[schema(as = ...)]` 加模块前缀，避免和别的模块的同名 schema 在
         *     openapi.json 里互相覆盖。
         */
        AdminMessageResponse: {
            message: string;
        };
        /** @description 数据库重置成功的响应。`warning` 给前端弹窗展示「数据已清空」。 */
        AdminResetDatabaseResponse: {
            message: string;
            warning: string;
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
        /** @description 请求里的一项。下单与报价的请求体同形，所以只有这一个结构。 */
        CartItemRequest: {
            /** Format: int64 */
            product_id: number;
            /** Format: int64 */
            quantity: number;
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
        ClosingCountRow: {
            /** Format: int64 */
            counted_qty: number;
            /** Format: int64 */
            event_product_id: number;
        };
        ClosingDiffRow: {
            /** Format: int64 */
            book_qty: number;
            /** Format: int64 */
            counted_qty: number;
            /** Format: int64 */
            event_product_id: number;
            name: string;
        };
        ClosingOnSiteRow: {
            /** Format: int64 */
            event_product_id: number;
            name: string;
            owner_name: string;
            /** Format: int64 */
            owner_society_id: number;
            product_code: string;
            /** Format: int64 */
            qty: number;
        };
        ClosingPendingOrderRow: {
            created_at: string;
            /** @description 分。与前端 `formatYuan` 的约定一致。 */
            final_amount: components["schemas"]["Money"];
            /** Format: int64 */
            id: number;
            /** Format: int64 */
            item_count: number;
        };
        ClosingState: {
            blockers: string[];
            onsite_remaining: components["schemas"]["ClosingOnSiteRow"][];
            pending_orders: components["schemas"]["ClosingPendingOrderRow"][];
            status: components["schemas"]["EventStatus"];
            stocktaken_at?: string | null;
        };
        /** @description 仅用于 OpenAPI 文档：创建展会的 multipart 表单字段。 */
        CreateEventForm: {
            date: string;
            location?: string | null;
            name: string;
            /** Format: binary */
            payment_qr_code_alipay?: string | null;
            /** Format: binary */
            payment_qr_code_wechat?: string | null;
            vendor_password?: string | null;
        };
        /** @description 仅用于 OpenAPI 文档：`create_product` 的 multipart 表单字段。 */
        CreateMasterProductForm: {
            category?: string | null;
            /** @description 元，不是分（老字段，`f64`）。 */
            default_price: string;
            /** Format: binary */
            image?: string | null;
            name: string;
            /** @description 所属社团 id；不传则归本社团。 */
            owner_society_id?: string | null;
            product_code: string;
            tags: string;
        };
        CreateOrderRequest: {
            items: components["schemas"]["CartItemRequest"][];
        };
        CreateSocietyRequest: {
            name: string;
        };
        CreatedEntry: {
            /** Format: int64 */
            id: number;
            /** Format: int64 */
            journal_id: number;
        };
        /** @description 两个全局密码是否仍是出厂默认值。 */
        DefaultPasswordsResponse: {
            /** @description 管理员密码仍是 `admin123` */
            admin: boolean;
            /** @description 全局摊主密码仍是 `vendor123`（它能进所有展会） */
            vendor: boolean;
        };
        /** @description 删除识别图的响应。 */
        DeleteMasterProductImageResponse: {
            /** Format: int64 */
            deleted_image_id: number;
            ok: boolean;
        };
        /** @description 删除社团的响应体。 */
        DeleteSocietyResponse: {
            message: string;
        };
        Event: {
            date: string;
            /** Format: int64 */
            id: number;
            location?: string | null;
            name: string;
            payment_qr_code_path?: string | null;
            status: components["schemas"]["EventStatus"];
        };
        /** @description `DELETE /events/{id}` 的成功响应体。 */
        EventDeletedResponse: {
            message: string;
        };
        EventResponse: {
            date: string;
            /** Format: int64 */
            id: number;
            location?: string | null;
            name: string;
            /** @description 向后兼容：保留单个 URL（取第一个），旧前端不会崩 */
            qrcode_url?: string | null;
            /** @description 新字段：所有收款码 URL 数组 */
            qrcode_urls: string[];
            status: components["schemas"]["EventStatus"];
        };
        /**
         * @description 展会状态。
         * @enum {string}
         */
        EventStatus: "筹备" | "进行中" | "已结算";
        EventUpdateStatusRequest: {
            status: components["schemas"]["EventStatus"];
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
        /** @description 仅用于 OpenAPI 文档：multipart 表单的字段。 */
        ImportProductsForm: {
            /**
             * Format: binary
             * @description `.boothpack` / `.zip` 文件内容。
             */
            file: string;
        };
        /**
         * @description 一条登记记录。`vendor_paid` 由「这条 journal 有没有资金腿」推出来，
         *     不另存一列——存两处就会有一处先腐烂。
         */
        InventoryLogEntry: {
            /** Format: int64 */
            event_product_id: number;
            /** Format: int64 */
            journal_id: number;
            name: string;
            note?: string | null;
            occurred_at: string;
            owner_name: string;
            /** Format: int64 */
            owner_society_id: number;
            product_code: string;
            /** Format: int64 */
            qty: number;
            vendor_paid: boolean;
        };
        InventoryLogRequest: {
            /** Format: int64 */
            event_product_id: number;
            note?: string | null;
            /** Format: int64 */
            qty: number;
            /**
             * @description 只对赠送有意义。报废时忽略——报废的货没有「谁买单」这回事，
             *     真要赔给货主是结算调整的事（协商结果，系统推不出来）。
             */
            vendor_pays?: boolean;
        };
        InventoryLogResponse: {
            /** Format: int64 */
            journal_id: number;
        };
        IsDefaultAdminPasswordResponse: {
            is_default: boolean;
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
        /** @description `/status` 的响应。计数是给前端「有没有值得打扰用户的历史数据」判断用的。 */
        LegacyStatus: {
            /** Format: int64 */
            event_count: number;
            has_backup: boolean;
            /**
             * Format: int64
             * @description `order_items` 的**明细行数**，不是 `SUM(quantity)` 的件数——别拿它写
             *     「你有 N 件历史商品」这类文案。
             */
            item_count: number;
            /** Format: int64 */
            order_count: number;
        };
        LoginRequest: {
            /** Format: int64 */
            eventId?: number | null;
            password: string;
            role: string;
        };
        LoginResponse: {
            access: string;
            /** Format: int64 */
            eventId?: number | null;
            message: string;
            role: string;
            token: string;
        };
        /** @description 退出登录的成功体。形状和登录响应里的 `message` 一样，单独建类型只是为了文档。 */
        LogoutResponse: {
            message: string;
        };
        LotImportItem: {
            /**
             * Format: int64
             * @description 源套装 id（**必须属于别的展会**）。
             */
            source_lot_id: number;
        };
        LotPayload: {
            /**
             * @description 老客户端不带这个字段，`serde(default)` 落到 false——**默认限 1**，
             *     不会因为升级悄悄从「每种 1 件」变成「可同款」。
             */
            allow_repeat?: boolean;
            candidate_ids: number[];
            name: string;
            /** Format: int64 */
            pick_count: number;
            /** @description 单位：分。 */
            total_price: components["schemas"]["Money"];
        };
        LotPreviewCandidate: {
            name: string;
            /** Format: int64 */
            owner_society_id: number;
            owner_society_name: string;
            /** Format: int64 */
            product_id: number;
            /** @description 单位：分。 */
            unit_price: components["schemas"]["Money"];
        };
        LotPreviewMember: {
            name: string;
            /** Format: int64 */
            product_id: number;
            /** Format: int64 */
            qty: number;
        };
        /** @description 试算请求：一份**还没保存**的套装配置。字段与 `LotPayload` 相同，只是不要名字。 */
        LotPreviewRequest: {
            allow_repeat?: boolean;
            candidate_ids: number[];
            /** Format: int64 */
            pick_count: number;
            /** @description 单位：分。 */
            total_price: components["schemas"]["Money"];
        };
        LotPreviewResponse: {
            candidates: components["schemas"]["LotPreviewCandidate"][];
            scenarios: components["schemas"]["LotPreviewScenario"][];
            warnings: components["schemas"]["LotPreviewWarning"][];
        };
        /** @description 顾客可能怎么凑满这个套装的一个极端。 */
        LotPreviewScenario: {
            /**
             * @description `original_amount − lot_price`。**可能为负**——那表示这种组合下套装比原价还贵，
             *     求解器不会套用它。故意保留负数而不是省略字段：调用方（含将来的 LLM）
             *     判一个符号，比判一个字段在不在要可靠。
             */
            discount: components["schemas"]["Money"];
            /** @description `"max_discount"`（顾客拿走最贵的那几件）或 `"min_discount"`（最便宜的那几件）。 */
            kind: string;
            /** @description 套装价（分），两个 scenario 相同，放进来是为了这个对象能独立读懂。 */
            lot_price: components["schemas"]["Money"];
            members: components["schemas"]["LotPreviewMember"][];
            /** @description 这些成分按原价的合计（分）。 */
            original_amount: components["schemas"]["Money"];
        };
        LotPreviewWarning: {
            /** @description 机器读的稳定标识；`message` 是给摊主看的，措辞会变，code 不会。 */
            code: string;
            message: string;
        };
        LotQuoteLine: {
            allocated_amount: components["schemas"]["Money"];
            /** @description 进了 `lots` 数组的第几个，`null` = 没进套装。 */
            lot_index?: number | null;
            /** Format: int64 */
            product_id: number;
            /** Format: int64 */
            qty: number;
        };
        LotQuoteLot: {
            /** Format: int64 */
            lot_id: number;
            members: components["schemas"]["LotQuoteMember"][];
            name: string;
            /**
             * @description 成分按原价的合计（分）。
             *
             *     前端显示「已应用：XX −YY」时 `YY = original_amount − price`。放在后端算，
             *     是因为前端再做一遍单价乘法就等于把分摊逻辑抄了半份出去。
             */
            original_amount: components["schemas"]["Money"];
            /** @description 套装价（分）。 */
            price: components["schemas"]["Money"];
        };
        LotQuoteMember: {
            /** Format: int64 */
            product_id: number;
            /** Format: int64 */
            qty: number;
        };
        LotQuoteRequest: {
            items: components["schemas"]["CartItemRequest"][];
        };
        LotQuoteResponse: {
            gross_amount: components["schemas"]["Money"];
            lines: components["schemas"]["LotQuoteLine"][];
            lots: components["schemas"]["LotQuoteLot"][];
            solved_amount: components["schemas"]["Money"];
        };
        LotResponse: {
            /** @description 一个套装实例里，同一个候选能不能算多件（默认 false = 每种最多 1 件）。 */
            allow_repeat: boolean;
            candidate_ids: number[];
            /** Format: int64 */
            event_id: number;
            /** Format: int64 */
            id: number;
            name: string;
            /**
             * Format: int64
             * @description 候选集必然同一货主（下面的 `validate_candidates` 保证），所以取其一即可。
             */
            owner_society_id: number;
            owner_society_name: string;
            /** Format: int64 */
            pick_count: number;
            /** @description 单位：分。 */
            total_price: components["schemas"]["Money"];
        };
        MasterProduct: {
            category?: string | null;
            /** Format: double */
            default_price: number;
            /** Format: int64 */
            id: number;
            /** Format: int64 */
            image_count?: number | null;
            image_url?: string | null;
            is_active: boolean;
            name: string;
            /**
             * Format: int64
             * @description 归属社团的**默认值**。选品时会被快照到 `event_products.owner_society_id`，
             *     之后改这里不影响已有展会的账（spec 3.1）。
             */
            owner_society_id?: number;
            product_code: string;
            tags?: string;
        };
        MasterProductImageDto: {
            created_at: string;
            has_embedding: boolean;
            /** Format: int64 */
            id: number;
            image_url: string;
            kind: string;
            /** Format: int64 */
            master_product_id: number;
        };
        /** @description 新增 / 修改识别图后返回的字段。 */
        MasterProductImageResponse: {
            /** Format: int64 */
            id: number;
            image_url: string;
            kind: string;
            /** Format: int64 */
            master_product_id: number;
        };
        MasterProductUpdateStatusRequest: {
            is_active: boolean;
        };
        /**
         * Format: cents
         * @description 金额，单位：分
         */
        Money: Cents;
        OrderItemResponse: {
            allocated_amount: components["schemas"]["Money"];
            /** Format: int64 */
            id: number;
            /**
             * @description 这一行进了哪个套装。`None` = 散卖。
             *
             *     同一个商品可能在一张订单里出现两次（2 件进套装、1 件散着，spec 4.5），
             *     摊主必须看得出哪一行是哪一种，否则配货时会以为系统重复计数了。
             */
            lot_name?: string | null;
            paid_amount: components["schemas"]["Money"];
            /** Format: int64 */
            product_id: number;
            product_image_url?: string | null;
            product_name: string;
            /** @description 单位：分（展示端除以 100 是前端 Task 8 的事）。 */
            product_price: components["schemas"]["Money"];
            /** Format: int64 */
            quantity: number;
            /** @description 这一行已经退给顾客多少钱（分，`SUM(refunds.refund_amount)`）。 */
            refunded_amount: components["schemas"]["Money"];
            /**
             * Format: int64
             * @description 这一行已经退了几件（`SUM(refunds.qty)`，无退货为 0）。
             */
            refunded_qty: number;
        };
        /**
         * @description 订单上的一个套装**实例**。
         *
         *     `id` 是 `order_lots.id` 而不是 `lots.id`——同一个套装可以套用多次
         *     （「任选3本100」买 6 本 = 两个实例），拆的时候必须能指到具体是哪一次。
         */
        OrderLotResponse: {
            /** Format: int64 */
            id: number;
            /**
             * Format: int64
             * @description 原始 Lot 的 id。套装被删掉后是 null（`ON DELETE SET NULL`），名字和价格仍在。
             */
            lot_id?: number | null;
            name: string;
            /**
             * @description 成分按原价的合计（分）。**摊主拆掉它时应收会回到这个数**，
             *     收款弹窗靠它在本地把新的应收算出来，不必多一次往返。
             */
            original_amount: components["schemas"]["Money"];
            /** @description 套装价（分），下单那一刻的快照。 */
            price: components["schemas"]["Money"];
        };
        /**
         * @description 订单响应：`{...order, items: [...]}`。
         *     `order` 是 `OrderRow` 摊平后的字段，`created_at` 经 serde rename 成 `timestamp`
         *     （前端读的是 timestamp，见 db/models.rs 的注释，别动）。
         */
        OrderResponse: components["schemas"]["OrderRow"] & {
            items: components["schemas"]["OrderItemResponse"][];
            lots: components["schemas"]["OrderLotResponse"][];
            /**
             * @description 这张订单已退给顾客的总额（分）= 其 items 的 `refunded_amount` 之和。
             *
             *     放在顶层而不是 `OrderRow`：`OrderRow` 是 `SELECT *` 的 `FromRow`，
             *     给它加列会直接炸（global.md 的实现层修正）。摊主列表按订单看「已退」，
             *     不该自己把行加一遍。
             */
            refunded_amount: components["schemas"]["Money"];
        };
        OrderRow: {
            channel?: string | null;
            /** Format: date-time */
            completed_at?: string | null;
            /** Format: int64 */
            event_id: number;
            final_amount: components["schemas"]["Money"];
            /** @description 以下三个单位都是分。②-1 里恒相等；②-2 引入 Lot 和手工覆盖后才会分开。 */
            gross_amount: components["schemas"]["Money"];
            /** Format: int64 */
            id: number;
            solved_amount: components["schemas"]["Money"];
            status: components["schemas"]["OrderStatus"];
            /**
             * Format: date-time
             * @description 前端读的是 `timestamp` —— 这个 rename 是个隐形契约，
             *     `frontend/src/components/order/OrderCard.vue:55` 和
             *     `frontend/src/views/AdminEventOrders.vue:114` 都依赖它。改名会静默白屏。
             */
            timestamp: string;
        };
        /**
         * @description 订单状态。
         * @enum {string}
         */
        OrderStatus: "pending" | "completed" | "cancelled";
        /**
         * @description 改订单状态的请求体。schema 名加 `Order` 前缀：`UpdateStatusRequest` 这种名字
         *     还会出现在别的模块，重名会在 openapi.json 里互相覆盖且不报错。
         */
        OrderUpdateStatusRequest: {
            channel?: string | null;
            final_amount?: components["schemas"]["Money"] | null;
            status: components["schemas"]["OrderStatus"];
            /**
             * @description 要拆掉的套装实例（`order_lots.id`）。
             *
             *     **拆 ≠ 改总价。** 改总价把差额记成手工折让、按 spec 4.4 整笔落本社团；
             *     而「这个套装不该套用」是纠错，钱必须回到真正的货主头上。代卖货上
             *     只改总价会让货主少拿钱、差额挂在本社团头上——金额总数对，归属错。
             */
            unapply_lot_ids?: number[] | null;
        };
        ProductAddRequest: {
            /** Format: int64 */
            initial_stock: number;
            product_code: string;
            unit_price?: components["schemas"]["Money"] | null;
        };
        /** @description 删除成功后的固定消息体。字段名 `message` 是既有响应形状的一部分。 */
        ProductDeleteMessage: {
            message: string;
        };
        /**
         * @description 响应体。**没有 `current_stock` / `initial_stock`**：
         *     `onsite_qty` 是聚合余额，`stocked_qty` 是累计进货（前端库存条的分母）。
         */
        ProductEventProduct: {
            category?: string | null;
            /** Format: int64 */
            event_id: number;
            /** Format: int64 */
            id: number;
            image_url?: string | null;
            /** Format: int64 */
            master_product_id: number;
            name: string;
            /** Format: int64 */
            onsite_qty: number;
            /** Format: int64 */
            owner_society_id: number;
            owner_society_name: string;
            product_code: string;
            /** Format: int64 */
            stocked_qty: number;
            tags: string;
            /** @description 单位：分。 */
            unit_price: components["schemas"]["Money"];
        };
        ProductImportItem: {
            /** Format: int64 */
            initial_stock: number;
            /** Format: int64 */
            master_product_id: number;
            /** @description 单位：分。 */
            unit_price: components["schemas"]["Money"];
        };
        ProductImportRequest: {
            /** @description 要从别的展会复制的套装。可省略（只上架商品时）。 */
            lots?: components["schemas"]["LotImportItem"][];
            /** @description 要上架的商品。可省略（只导入套装时）。 */
            products?: components["schemas"]["ProductImportItem"][];
        };
        ProductImportResponse: {
            lots: components["schemas"]["LotResponse"][];
            products: components["schemas"]["ProductEventProduct"][];
        };
        ProductRestockRequest: {
            note?: string | null;
            /** Format: int64 */
            qty: number;
        };
        ProductUpdateRequest: {
            unit_price?: components["schemas"]["Money"] | null;
        };
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
        /**
         * @description 退货的货去哪：`现场仓`（还能卖）或 `损耗`（已损坏）。
         * @enum {string}
         */
        RefundDestination: "现场仓" | "损耗";
        RefundHistoryRow: {
            channel: string;
            destination: components["schemas"]["RefundDestination"];
            /** Format: int64 */
            id: number;
            name: string;
            occurred_at: string;
            /** Format: int64 */
            order_line_id: number;
            product_code: string;
            /** Format: int64 */
            qty: number;
            refund_amount: components["schemas"]["Money"];
        };
        RefundLineRequest: {
            /** @description `现场仓`（还能卖）或 `损耗`（已损坏）。 */
            destination: components["schemas"]["RefundDestination"];
            /** Format: int64 */
            order_line_id: number;
            /** Format: int64 */
            qty: number;
        };
        RefundListResponse: {
            history: components["schemas"]["RefundHistoryRow"][];
            lines: components["schemas"]["RefundableLine"][];
        };
        RefundRequest: {
            channel: string;
            lines: components["schemas"]["RefundLineRequest"][];
            refund_amount?: components["schemas"]["Money"] | null;
        };
        RefundResponse: {
            allocated_total: components["schemas"]["Money"];
            /** Format: int64 */
            journal_id: number;
            paid_total: components["schemas"]["Money"];
            refund_amount: components["schemas"]["Money"];
        };
        /**
         * @description 可退的行。**同一个商品可能出现多行**（按 Lot 归属拆的），
         *     所以前端不要按 `event_product_id` 去重（②-2 交接契约第 3 条）。
         */
        RefundableLine: {
            /** Format: int64 */
            event_product_id: number;
            lot_name?: string | null;
            name: string;
            /** Format: int64 */
            order_line_id: number;
            product_code: string;
            /** Format: int64 */
            qty: number;
            /** Format: int64 */
            refunded_qty: number;
            remaining_allocated: components["schemas"]["Money"];
            remaining_paid: components["schemas"]["Money"];
            /** Format: int64 */
            remaining_qty: number;
        };
        /**
         * @description server-info 的响应：LAN 访问所需的 IP、端口与各入口 URL。
         *
         *     字段按字母序声明：`serde_json` 未开 `preserve_order`，原来的 `json!` 输出
         *     就是这个顺序，这样序列化出来的字节与迁移前一致。
         */
        ServerInfo: {
            /** @description 管理员入口。 */
            admin_url: string;
            /** @description API 根路径。 */
            api_base_url: string;
            /** @description LAN 访问的 HTTPS 根 URL。 */
            base_url: string;
            /** @description 给 LAN 设备用的 IP。 */
            ip: string;
            /** @description 顾客下单入口。 */
            order_url: string;
            /**
             * Format: int32
             * @description HTTPS 端口。
             */
            port: number;
            /** @description 摊主入口。 */
            vendor_url: string;
        };
        SettleResponse: {
            status: components["schemas"]["EventStatus"];
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
        Society: {
            /** Format: int64 */
            id: number;
            is_home: boolean;
            name: string;
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
        StatsProductSalesItem: {
            /**
             * Format: int64
             * @description 「累计进货」= 从外部进到现场仓的总件数（不是当前余额）。
             */
            initial_stock: number;
            product_code: string;
            /** Format: int64 */
            product_id: number;
            product_name: string;
            /** Format: int64 */
            total_quantity: number;
            /**
             * @description 单位：分。**顾客实付合计**（`Σ paid_amount`），不是原价合计——
             *     Lot 分摊与手工折让都已经摊进去了。
             */
            total_revenue_per_item: components["schemas"]["Money"];
            /** @description 单位：分。 */
            unit_price: components["schemas"]["Money"];
        };
        /** @description 仪表盘响应：展会信息 + 汇总 + 按商品明细。 */
        StatsResponse: {
            event_info: components["schemas"]["Event"];
            product_details: components["schemas"]["StatsProductSalesItem"][];
            summary: components["schemas"]["StatsSummary"];
        };
        /** @description 销售趋势响应。金额单位：分。 */
        StatsSalesResponse: {
            event_name: string;
            summary: components["schemas"]["StatsProductSalesItem"][];
            timeseries: components["schemas"]["StatsTimeseriesItem"][];
            /** @description 单位：分。 */
            total_revenue: components["schemas"]["Money"];
        };
        /** @description 仪表盘汇总。金额单位：分。 */
        StatsSummary: {
            /** Format: int64 */
            completed_orders_count: number;
            /** Format: int64 */
            total_items_sold: number;
            /** @description 单位：分。 */
            total_revenue: components["schemas"]["Money"];
        };
        /** @description 趋势图上的一个时间桶。金额单位：分。 */
        StatsTimeseriesItem: {
            date: string;
            /** @description 单位：分。 */
            revenue: components["schemas"]["Money"];
        };
        StocktakeRequest: {
            counts: components["schemas"]["ClosingCountRow"][];
        };
        StocktakeResponse: {
            diffs: components["schemas"]["ClosingDiffRow"][];
            /**
             * Format: int64
             * @description 零差异时为 `None`——`post_journal` 拒绝空 journal，而
             *     「盘了全对」这个事实靠 `events.stocktaken_at` 记，不靠 journal 存在性。
             */
            journal_id?: number | null;
        };
        /** @description 导入成功后的响应体。 */
        SyncImportResponse: {
            images_count: number;
            message: string;
            products_count: number;
        };
        TakebackResponse: {
            /** Format: int64 */
            journal_id?: number | null;
            /** Format: int64 */
            moved: number;
        };
        UpdateAdminPasswordRequest: {
            newPassword: string;
            oldPassword: string;
        };
        /** @description 仅用于 OpenAPI 文档：更新展会的 multipart 表单字段。 */
        UpdateEventForm: {
            date?: string | null;
            location?: string | null;
            name?: string | null;
            /** Format: binary */
            payment_qr_code_alipay?: string | null;
            /** Format: binary */
            payment_qr_code_wechat?: string | null;
            remove_payment_qr_code?: boolean | null;
            vendor_password?: string | null;
        };
        /** @description 仅用于 OpenAPI 文档：`update_product` 的 multipart 表单字段。 */
        UpdateMasterProductForm: {
            category?: string | null;
            /** @description 元，不是分（老字段，`f64`）。 */
            default_price?: string | null;
            /** Format: binary */
            image?: string | null;
            name?: string | null;
            /** @description 所属社团 id；不传则不变。只影响之后上架到展会的货。 */
            owner_society_id?: string | null;
            product_code?: string | null;
            remove_image?: boolean | null;
            tags?: string | null;
        };
        /** @description 仅用于 OpenAPI 文档：`update_product_image` 的 multipart 表单字段。 */
        UpdateProductImageForm: {
            /** Format: binary */
            image?: string | null;
            kind?: string | null;
        };
        UpdateSocietyRequest: {
            is_home?: boolean | null;
            name?: string | null;
        };
        UpdateVendorPasswordRequest: {
            newPassword: string;
        };
        VisionActivateModelRequest: {
            model_id: string;
        };
        VisionActivateModelResponse: {
            active_model_id: string;
            ok: boolean;
            rebuild_started: boolean;
        };
        VisionDeleteModelResponse: {
            deleted_model_id: string;
            ok: boolean;
        };
        VisionEpSettingResponse: {
            active: string;
            configured: string;
            gpu_devices: components["schemas"]["VisionGpuDevice"][];
            platform: string;
        };
        VisionGpuDevice: {
            /** Format: int32 */
            device_id: number;
            name: string;
        };
        VisionInstallModelRequest: {
            model_id: string;
            source?: string | null;
        };
        VisionInstallModelResponse: {
            model_id: string;
            ok: boolean;
            source: string;
            task_id: string;
        };
        VisionInstallTaskResponse: {
            error?: string | null;
            message?: string | null;
            model_id: string;
            /** Format: int32 */
            progress: number;
            status: string;
            task_id: string;
        };
        VisionModelItem: {
            description?: string | null;
            dim: number;
            input_size: number;
            installed: boolean;
            is_active: boolean;
            model_id: string;
            model_version: string;
            /** Format: double */
            size_mb?: number | null;
            tier?: string | null;
        };
        VisionModelsResponse: {
            active_model_id: string;
            models: components["schemas"]["VisionModelItem"][];
        };
        VisionRebuildRequest: {
            force_full?: boolean | null;
        };
        VisionRebuildResponse: {
            force_full: boolean;
            message: string;
            ok: boolean;
        };
        /** @description 仅用于 OpenAPI 文档：`/search` 的 multipart 表单字段。 */
        VisionSearchForm: {
            /** @description mode 为 order/admin_event 时必填。 */
            event_id?: string | null;
            /**
             * Format: binary
             * @description 查询图片，必填。
             */
            image: string;
            /** @description JSON 数组或逗号分隔的商品 id。 */
            master_product_ids?: string | null;
            /** @description `order` | `admin_event` | `admin_master`。 */
            mode?: string | null;
            /** @description ROI JSON：`{"x":..,"y":..,"w":..,"h":..}`。 */
            roi?: string | null;
            /** @description 返回前 K 个结果，1..=20，缺省 5。 */
            top_k?: string | null;
        };
        VisionSearchResponse: {
            /** Format: int64 */
            index_version: number;
            is_uncertain: boolean;
            model_id: string;
            model_version: string;
            results: components["schemas"]["VisionSearchResult"][];
        };
        VisionSearchResult: {
            /** Format: int64 */
            master_product_id: number;
            name: string;
            product_code: string;
            /** Format: float */
            score: number;
            thumb_url?: string | null;
        };
        VisionSetEpRequest: {
            execution_provider: string;
        };
        VisionSetEpResponse: {
            active: string;
            execution_provider: string;
            message: string;
            ok: boolean;
        };
        VisionStatusResponse: {
            execution_provider: string;
            /** Format: int64 */
            index_size: number;
            /** Format: int64 */
            index_version: number;
            is_ready: boolean;
            is_rebuilding: boolean;
            last_rebuild_at?: string | null;
            /** @description 最近一次重建失败的原因（下一次成功后清空） */
            last_rebuild_error?: string | null;
            model_id: string;
            model_version: string;
            reason?: string | null;
            /** Format: int64 */
            rebuild_processed: number;
            /** Format: int64 */
            rebuild_total: number;
            /** @description ONNX Runtime 加载失败的原因；有值时识别整体不可用，`is_ready` 恒为 false */
            runtime_error?: string | null;
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
    "admin.default_passwords": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DefaultPasswordsResponse"];
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
            /** @description 需要管理员 */
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
    "admin.update_admin_password": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateAdminPasswordRequest"];
            };
        };
        responses: {
            /** @description 管理员密码已更新 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AdminMessageResponse"];
                };
            };
            /** @description 新密码少于 4 位 */
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
            /** @description 需要管理员 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description settings 缺行或数据库写入失败（部分分支为纯文本） */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    "admin.reset_database_handler": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 数据库已完全重置 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AdminResetDatabaseResponse"];
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
            /** @description 需要管理员 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 清理上传目录或数据库事务失败（部分分支为纯文本） */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    "admin.update_vendor_default_password": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateVendorPasswordRequest"];
            };
        };
        responses: {
            /** @description 默认摊主密码已更新 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AdminMessageResponse"];
                };
            };
            /** @description 新密码少于 4 位 */
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
            /** @description 需要管理员 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 数据库写入失败（返回纯文本，非 error 形状） */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    "auth.is_default_admin_password": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 未设置密码时也返回 false */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["IsDefaultAdminPasswordResponse"];
                };
            };
        };
    };
    "auth.login_handler": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["LoginRequest"];
            };
        };
        responses: {
            /** @description 登录成功；同时 Set-Cookie 下发 access_token_cookie */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["LoginResponse"];
                };
            };
            /** @description 请求体不是合法 JSON：纯文本 `JSON Parse Error: …` */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
            /** @description 角色或密码错误 */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 令牌创建失败 */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    "auth.logout_handler": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 已退出；同时 Set-Cookie 清除 access_token_cookie */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["LogoutResponse"];
                };
            };
        };
    };
    "settlement.list_channels": {
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
    "event.list_events": {
        parameters: {
            query?: {
                /** @description 只列这个状态的；不传则全部。 */
                status?: components["schemas"]["EventStatus"] | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 按 event_date 倒序 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["EventResponse"][];
                };
            };
        };
    };
    "event.create_event": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "multipart/form-data": components["schemas"]["CreateEventForm"];
            };
        };
        responses: {
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["EventResponse"];
                };
            };
            /** @description 缺少 name 或 date */
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
            /** @description 需要管理员 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 保存上传文件或写库失败 */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    "settlement.list_adjustments": {
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
    "settlement.create_adjustment": {
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
    "settlement.delete_adjustment": {
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
    "settlement.list_advances": {
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
    "settlement.create_advance": {
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
    "settlement.delete_advance": {
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
    "closing.get_closing": {
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
                    "application/json": components["schemas"]["ClosingState"];
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
    "closing.settle": {
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
                    "application/json": components["schemas"]["SettleResponse"];
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
            /** @description 还有待处理订单、现场仓还有货，或展会已结算 */
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
    "closing.stocktake": {
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
                "application/json": components["schemas"]["StocktakeRequest"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["StocktakeResponse"];
                };
            };
            /** @description 重复报数、实数为负、漏报现场仓商品或商品不在这场展会 */
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
            /** @description 还有待处理订单，或展会已结算 */
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
    "closing.takeback": {
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
                    "application/json": components["schemas"]["TakebackResponse"];
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
            /** @description 还有待处理订单、展会已结算或账本不自洽 */
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
    "inventory.list_gifts": {
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
            /** @description 按 journal id 倒序 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["InventoryLogEntry"][];
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
    "inventory.create_gift": {
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
                "application/json": components["schemas"]["InventoryLogRequest"];
            };
        };
        responses: {
            /** @description 登记成功 */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["InventoryLogResponse"];
                };
            };
            /** @description 数量非正或金额溢出 */
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
            /** @description 商品不在这场展会里，或展会不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 现场仓余量不足，或展会已结算 */
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
    "inventory.reverse_entry": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 展会 id */
                event_id: number;
                /** @description 要撤销的 journal id */
                journal_id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 撤销成功，返回冲正 journal 的 id */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["InventoryLogResponse"];
                };
            };
            /** @description 这条 journal 不是赠送或报废 */
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
            /** @description 记录不存在，或展会不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 这条记录已经撤销过了，或展会已结算 */
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
    "lot.list_lots": {
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
            /** @description 按 id 升序；每个套装的候选按 id 升序 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["LotResponse"][];
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
    "lot.create_lot": {
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
                "application/json": components["schemas"]["LotPayload"];
            };
        };
        responses: {
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["LotResponse"];
                };
            };
            /** @description 名称/件数/价格不合法、候选不属于本场或跨货主、不允许同款但候选种类不够 */
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
            /** @description 展会已结算，不能再改账 */
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
    "lot.preview_lot": {
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
                "application/json": components["schemas"]["LotPreviewRequest"];
            };
        };
        responses: {
            /** @description 零写入的试算结果 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["LotPreviewResponse"];
                };
            };
            /** @description 件数/价格不合法、候选不属于本场或跨货主、不允许同款但候选种类不够 */
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
        };
    };
    "lot.update_lot": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 展会 id */
                event_id: number;
                /** @description 套装 id */
                lot_id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["LotPayload"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["LotResponse"];
                };
            };
            /** @description 名称/件数/价格不合法、候选不属于本场或跨货主、不允许同款但候选种类不够 */
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
            /** @description 套装或展会不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 展会已结算，不能再改账 */
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
    "lot.delete_lot": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 展会 id */
                event_id: number;
                /** @description 套装 id */
                lot_id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 已删除 */
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
            /** @description 套装或展会不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 展会已结算，不能再改账 */
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
    "order.list_orders": {
        parameters: {
            query?: {
                /** @description 只列这个状态的；不传则全部。 */
                status?: components["schemas"]["OrderStatus"] | null;
            };
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
                    "application/json": components["schemas"]["OrderResponse"][];
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
    "order.create_order": {
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
                "application/json": components["schemas"]["CreateOrderRequest"];
            };
        };
        responses: {
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["OrderResponse"];
                };
            };
            /** @description 购物车为空、数量非正或金额溢出 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 展会或商品不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 库存不足，或展会不是「进行中」 */
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
    "refund.list_refunds": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 展会 id */
                event_id: number;
                /** @description 订单 id */
                order_id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 退货历史与可退行 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["RefundListResponse"];
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
            /** @description 订单不存在或不属于这场展会 */
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
    "refund.create_refund": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 展会 id */
                event_id: number;
                /** @description 订单 id */
                order_id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["RefundRequest"];
            };
        };
        responses: {
            /** @description 退货已记账 */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["RefundResponse"];
                };
            };
            /** @description 渠道为空、退行重复、去向不认识、数量非正、退款为负或超过实付、订单状态不对、订单行不属于该订单 */
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
            /** @description 订单不存在或展会不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 可退数量不足、没有本社团、或展会已结算 */
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
    "order.update_order_status": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 展会 id */
                event_id: number;
                /** @description 订单 id */
                order_id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["OrderUpdateStatusRequest"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["OrderResponse"];
                };
            };
            /** @description 未知状态、实收为负、缺渠道或要拆的套装不属于这张订单 */
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
            /** @description 订单不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 订单已取消/已完成，或展会已结算 */
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
    "product.list_event_products": {
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
                    "application/json": components["schemas"]["ProductEventProduct"][];
                };
            };
        };
    };
    "product.add_product_to_event": {
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
                "application/json": components["schemas"]["ProductAddRequest"];
            };
        };
        responses: {
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ProductEventProduct"];
                };
            };
            /** @description 进货数量或单价为负 */
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
            /** @description 展会不存在或商品编号不在全局商品库 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 该商品已经在本场展会或展会已结算 */
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
    "product.import_products": {
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
                "application/json": components["schemas"]["ProductImportRequest"];
            };
        };
        responses: {
            /** @description 整批成功；商品与新建套装的完整响应 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ProductImportResponse"];
                };
            };
            /** @description 请求为空、单价/库存为负、源套装属于本场或候选商品映射不到 */
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
            /** @description 展会或源套装不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 商品已在本场（不跳过）或展会已结算 */
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
    "product.restock_product": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 展会 id */
                event_id: number;
                /** @description 场次商品 id */
                id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ProductRestockRequest"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ProductEventProduct"];
                };
            };
            /** @description 补货数量必须为正 */
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
            /** @description 商品不存在或不属于这场展会 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 展会已结算 */
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
    "lot.quote": {
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
                "application/json": components["schemas"]["LotQuoteRequest"];
            };
        };
        responses: {
            /** @description 公开、无鉴权、零写入的报价；不查库存 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["LotQuoteResponse"];
                };
            };
            /** @description 购物车为空/过多、数量非正、金额溢出或组合方式过多 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 展会或商品不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 展会不是「进行中」 */
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
    "stats.get_sales_summary": {
        parameters: {
            query?: {
                product_code?: string | null;
                start_date?: string | null;
                end_date?: string | null;
                interval_minutes?: number | null;
            };
            header?: never;
            path: {
                /** @description 展会 id */
                event_id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 销售趋势 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["StatsSalesResponse"];
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
    "stats.download_sales_summary": {
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
            /** @description 销售记录 xlsx */
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
            /** @description 生成或读取 Excel 失败 */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    "inventory.list_scraps": {
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
            /** @description 按 journal id 倒序 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["InventoryLogEntry"][];
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
    "inventory.create_scrap": {
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
                "application/json": components["schemas"]["InventoryLogRequest"];
            };
        };
        responses: {
            /** @description 登记成功 */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["InventoryLogResponse"];
                };
            };
            /** @description 数量非正或金额溢出 */
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
            /** @description 商品不在这场展会里，或展会不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 现场仓余量不足，或展会已结算 */
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
    "settlement.get_settlement": {
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
    "settlement.download_settlement_xlsx": {
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
    "settlement.reconcile": {
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
    "stats.get_event_stats": {
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
            /** @description 仪表盘统计 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["StatsResponse"];
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
    "event.get_event": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 展会 id */
                id: number;
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
                    "application/json": components["schemas"]["EventResponse"];
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
    "event.update_event.put": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 展会 id */
                id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "multipart/form-data": components["schemas"]["UpdateEventForm"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["EventResponse"];
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
            /** @description 需要管理员 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 展会不存在（现状为纯文本响应） */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 上传或写库失败 */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    "event.update_event.post": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 展会 id */
                id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "multipart/form-data": components["schemas"]["UpdateEventForm"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["EventResponse"];
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
            /** @description 需要管理员 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 展会不存在（现状为纯文本响应） */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 上传或写库失败 */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    "event.delete_event": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 展会 id */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 删除成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["EventDeletedResponse"];
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
            /** @description 需要管理员 */
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
            /** @description 数据库错误 */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    "event.update_status": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 展会 id */
                id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["EventUpdateStatusRequest"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["EventResponse"];
                };
            };
            /** @description 状态值不合法，或试图直接改成已结算 */
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
            /** @description 需要管理员 */
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
            /** @description 已结算的展会不能解冻 */
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
    "legacy.export_legacy_xlsx": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 三张 sheet 的 xlsx 二进制流 */
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
            /** @description 需要管理员 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 没有可导出的历史数据 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
            /** @description 生成或读回 xlsx 失败 */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
        };
    };
    "legacy.get_legacy_status": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 备份是否存在，以及老库里的展会 / 订单 / 明细行数 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["LegacyStatus"];
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
            /** @description 需要管理员 */
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
    "master_product.list_products": {
        parameters: {
            query?: {
                all?: boolean | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["MasterProduct"][];
                };
            };
        };
    };
    "master_product.create_product": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "multipart/form-data": components["schemas"]["CreateMasterProductForm"];
            };
        };
        responses: {
            /** @description 创建成功 */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["MasterProduct"];
                };
            };
            /** @description 缺少 product_code 或 name */
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
            /** @description 需要管理员 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description product_code 已存在 */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 上传或数据库错误（纯文本） */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
        };
    };
    "master_product.update_product.put": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 商品 id */
                id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "multipart/form-data": components["schemas"]["UpdateMasterProductForm"];
            };
        };
        responses: {
            /** @description 更新成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["MasterProduct"];
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
            /** @description 需要管理员 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 商品不存在（纯文本） */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
            /** @description product_code 已存在 */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 数据库错误（纯文本） */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
        };
    };
    "master_product.update_product.post": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 商品 id */
                id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "multipart/form-data": components["schemas"]["UpdateMasterProductForm"];
            };
        };
        responses: {
            /** @description 更新成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["MasterProduct"];
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
            /** @description 需要管理员 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 商品不存在（纯文本） */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
            /** @description product_code 已存在 */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 数据库错误（纯文本） */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
        };
    };
    "master_product.list_product_images": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 商品 id */
                id: number;
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
                    "application/json": components["schemas"]["MasterProductImageDto"][];
                };
            };
            /** @description 数据库错误（纯文本） */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
        };
    };
    "master_product.add_product_image": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 商品 id */
                id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "multipart/form-data": components["schemas"]["AddProductImageForm"];
            };
        };
        responses: {
            /** @description 添加成功 */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["MasterProductImageResponse"];
                };
            };
            /** @description 缺少 image 字段 */
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
            /** @description 需要管理员 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 上传或数据库错误 */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    "master_product.update_product_image.put": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 商品 id */
                id: number;
                /** @description 识别图 id */
                image_id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "multipart/form-data": components["schemas"]["UpdateProductImageForm"];
            };
        };
        responses: {
            /** @description 更新成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["MasterProductImageResponse"];
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
            /** @description 需要管理员 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 识别图不存在（纯文本） */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
            /** @description 上传或数据库错误 */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    "master_product.update_product_image.post": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 商品 id */
                id: number;
                /** @description 识别图 id */
                image_id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "multipart/form-data": components["schemas"]["UpdateProductImageForm"];
            };
        };
        responses: {
            /** @description 更新成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["MasterProductImageResponse"];
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
            /** @description 需要管理员 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 识别图不存在（纯文本） */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
            /** @description 上传或数据库错误 */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    "master_product.delete_product_image": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 商品 id */
                id: number;
                /** @description 识别图 id */
                image_id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 删除成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DeleteMasterProductImageResponse"];
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
            /** @description 需要管理员 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 识别图不存在（纯文本） */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
            /** @description 数据库错误 */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    "master_product.update_status": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 商品 id */
                id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["MasterProductUpdateStatusRequest"];
            };
        };
        responses: {
            /** @description 更新成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["MasterProduct"];
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
            /** @description 需要管理员 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 数据库错误（纯文本） */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
        };
    };
    "product.update_product": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 场次商品 id */
                id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ProductUpdateRequest"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ProductEventProduct"];
                };
            };
            /** @description 单价不能为负 */
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
            /** @description 商品不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 展会已结算 */
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
    "product.delete_product": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 场次商品 id */
                id: number;
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
                    "application/json": components["schemas"]["ProductDeleteMessage"];
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
            /** @description 商品不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 商品已有进出记录或展会已结算 */
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
    "info.server_info_handler": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description LAN 的 IP、端口与各入口 URL */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ServerInfo"];
                };
            };
        };
    };
    "society.list_societies": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description 本社团在前，其余按名字 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Society"][];
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
        };
    };
    "society.create_society": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateSocietyRequest"];
            };
        };
        responses: {
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Society"];
                };
            };
            /** @description 社团名不能为空 */
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
            /** @description 需要管理员 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 社团已存在 */
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
    "society.update_society": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 社团 id */
                id: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateSocietyRequest"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Society"];
                };
            };
            /** @description 社团名为空，或试图取消本社团标记 */
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
            /** @description 需要管理员 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 社团不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 社团名已存在 */
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
    "society.delete_society": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 社团 id */
                id: number;
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
                    "application/json": components["schemas"]["DeleteSocietyResponse"];
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
            /** @description 需要管理员 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 社团不存在 */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 本社团，或仍被商品 / 垫付 / 结算调整引用 */
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
    "sync.export_products": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/zip": number[];
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
            /** @description 需要管理员 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 读取数据或打包失败 */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
        };
    };
    "sync.import_products": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "multipart/form-data": components["schemas"]["ImportProductsForm"];
            };
        };
        responses: {
            /** @description 导入成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["SyncImportResponse"];
                };
            };
            /** @description 没有 file 字段或读取上传字节失败 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
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
            /** @description 需要管理员 */
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
    "sync.import_products_raw": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/octet-stream": number[];
            };
        };
        responses: {
            /** @description 导入成功 */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["SyncImportResponse"];
                };
            };
            /** @description zip 或 catalog.json 无效 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
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
            /** @description 需要管理员 */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 数据库写入或 zip 处理失败 */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
                };
            };
        };
    };
    "vision.list_models": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["VisionModelsResponse"];
                };
            };
        };
    };
    "vision.activate_model": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["VisionActivateModelRequest"];
            };
        };
        responses: {
            202: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["VisionActivateModelResponse"];
                };
            };
            /** @description 模型不存在或未安装 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    "vision.install_model": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["VisionInstallModelRequest"];
            };
        };
        responses: {
            202: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["VisionInstallModelResponse"];
                };
            };
            /** @description source 不受支持、模型不存在或已安装 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    "vision.get_install_task": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 安装任务 id */
                task_id: string;
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
                    "application/json": components["schemas"]["VisionInstallTaskResponse"];
                };
            };
            /** @description 任务不存在 */
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
    "vision.delete_model": {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description 模型 id */
                model_id: string;
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
                    "application/json": components["schemas"]["VisionDeleteModelResponse"];
                };
            };
            /** @description 不能删除激活模型、模型不存在或未安装 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    "vision.rebuild_index": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: {
            content: {
                "application/json": components["schemas"]["VisionRebuildRequest"] | null;
            };
        };
        responses: {
            202: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["VisionRebuildResponse"];
                };
            };
            /** @description 索引正在重建 */
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
    "vision.search_by_image": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "multipart/form-data": components["schemas"]["VisionSearchForm"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["VisionSearchResponse"];
                };
            };
            /** @description 缺少图片、mode/roi/master_product_ids 非法 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 识别超时 */
            408: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 上传或推理失败 */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 索引重建中、未就绪或并发已满 */
            503: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    "vision.get_ep_setting": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["VisionEpSettingResponse"];
                };
            };
        };
    };
    "vision.set_ep_setting": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["VisionSetEpRequest"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["VisionSetEpResponse"];
                };
            };
            /** @description EP 取值非法 */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
            /** @description 配置保存或加载失败 */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorBody"];
                };
            };
        };
    };
    "vision.get_vision_status": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["VisionStatusResponse"];
                };
            };
        };
    };
}
