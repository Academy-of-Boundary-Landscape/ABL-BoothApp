// openapi.json → src/api/schema.d.ts。生成物入库，CI 会重跑本脚本并 git diff。
//
// 唯一的定制：`format: cents` 的 schema（Rust 的 Money）映射成 branded `Cents`，
// 让「把分当成元显示」在 TS 里报错。见 src/utils/money.ts。
import { writeFileSync } from 'node:fs'
import openapiTS, { astToString } from 'openapi-typescript'
import ts from 'typescript'

const CENTS = ts.factory.createTypeReferenceNode(ts.factory.createIdentifier('Cents'))

const ast = await openapiTS(new URL('../../src-tauri/openapi.json', import.meta.url), {
  transform(schemaObject) {
    if (schemaObject.format === 'cents') return CENTS
  },
})

const header = `/* eslint-disable */
// 由 frontend/scripts/gen-api.mjs 从 src-tauri/openapi.json 生成。
// 不要手改——改后端，UPDATE_OPENAPI=1 跑 cargo test，再 npm run gen:api。
import type { Cents } from '@/utils/money'

`
writeFileSync(new URL('../src/api/schema.d.ts', import.meta.url), header + astToString(ast))
