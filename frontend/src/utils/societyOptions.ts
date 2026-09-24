import type { components } from '@/api/schema'

type Society = components['schemas']['Society']

/** 社团下拉的选项：本社团排第一并标注，其余保持后端给的顺序。 */
export function societyOptions(list: Society[]): { label: string; value: number }[] {
  const home = list.filter((s) => s.is_home)
  const others = list.filter((s) => !s.is_home)
  return [
    ...home.map((s) => ({ label: `${s.name}（本社团）`, value: s.id })),
    ...others.map((s) => ({ label: s.name, value: s.id })),
  ]
}
