<template>
  <div class="chart-wrapper">
    <svg :viewBox="`0 0 ${width} ${height}`" preserveAspectRatio="xMidYMid meet">
      <defs>
        <linearGradient id="revenueGradient" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stop-color="var(--accent-color)" stop-opacity="0.35" />
          <stop offset="100%" stop-color="var(--accent-color)" stop-opacity="0" />
        </linearGradient>
      </defs>

      <g class="grid-lines">
        <line v-for="tick in yTicks" :key="`y-${tick}`" :x1="padding" :x2="width - padding" :y1="yForValue(tick)" :y2="yForValue(tick)" />
      </g>
      <g class="y-ticks">
        <text
          v-for="tick in yTicks"
          :key="`y-label-${tick}`"
          :x="padding - 10"
          :y="yForValue(tick)"
          text-anchor="end"
          dominant-baseline="middle"
        >
          {{ currency(tick) }}
        </text>
      </g>

      <path class="area" :d="areaPath" />
      <path class="line" :d="linePath" />

      <g class="points">
        <g v-for="(p, idx) in points" :key="p.label">
          <circle :cx="p.x" :cy="p.y" r="4" @mouseenter="onEnter(p)" @mouseleave="onLeave" />
          <text v-if="idx % labelStep === 0 || idx === points.length - 1" :x="p.x" :y="height - padding / 2" text-anchor="middle">{{ shortLabel(p.label) }}</text>
        </g>
      </g>
    </svg>

    <div v-if="hover" class="chart-tooltip" :style="tooltipStyle">
      <div class="tooltip-date">{{ hover.fullLabel }}</div>
      <div class="tooltip-value">{{ fullCurrency(hover.revenue) }}</div>
    </div>
  </div>
</template>

<script setup>
import { computed, ref } from 'vue';
import { formatChartLabel, formatChartTooltip } from '@/utils/dateFormatter';

const props = defineProps({
  series: { type: Array, default: () => [] },
  width: { type: Number, default: 800 },
  height: { type: Number, default: 320 },
  padding: { type: Number, default: 48 },
});

const hover = ref(null);

const maxRevenue = computed(() => Math.max(...(props.series?.map(p => p.revenue) || [0]), 1));

// Y 轴刻度取整
const yTicks = computed(() => {
  const max = maxRevenue.value;
  if (max <= 0) return [0];
  // 找一个漂亮的步长：1, 2, 5, 10, 20, 50, 100, ...
  const raw = max / 4;
  const mag = Math.pow(10, Math.floor(Math.log10(raw)));
  const nice = [1, 2, 5, 10].map(m => m * mag).find(s => s >= raw) || raw;
  const ticks = [];
  for (let v = 0; v <= max + nice * 0.01; v += nice) {
    ticks.push(Math.round(v * 100) / 100);
  }
  return ticks;
});

const effectiveMax = computed(() => {
  const ticks = yTicks.value;
  return ticks.length ? ticks[ticks.length - 1] : maxRevenue.value;
});

const stepX = computed(() => {
  const len = props.series?.length || 0;
  if (len <= 1) return 0;
  return (props.width - props.padding * 2) / (len - 1);
});

// X 轴标签间隔：避免重叠，大约每 80px 一个标签
const labelStep = computed(() => {
  const len = props.series?.length || 0;
  if (len <= 1) return 1;
  const availWidth = props.width - props.padding * 2;
  return Math.max(1, Math.ceil(len / (availWidth / 80)));
});

const points = computed(() => {
  const len = props.series?.length || 0;
  const centerX = props.width / 2;
  return (props.series || []).map((p, idx) => ({
    x: len === 1 ? centerX : props.padding + stepX.value * idx,
    y: props.padding + (props.height - props.padding * 2) * (1 - p.revenue / effectiveMax.value),
    label: formatChartLabel(p.date),
    fullLabel: formatChartTooltip(p.date),
    rawDate: p.date,
    revenue: p.revenue,
  }));
});

const linePath = computed(() => {
  if (!points.value.length) return '';
  const coords = points.value.map(p => `${p.x} ${p.y}`).join(' L ');
  return `M ${coords}`;
});

const areaPath = computed(() => {
  if (!points.value.length) return '';
  const start = `M ${points.value[0].x} ${props.height - props.padding}`;
  const line = points.value.map(p => `L ${p.x} ${p.y}`).join(' ');
  const end = `L ${points.value[points.value.length - 1].x} ${props.height - props.padding} Z`;
  return `${start} ${line} ${end}`;
});

function yForValue(value) {
  const max = effectiveMax.value;
  if (!max) return props.height - props.padding;
  return props.padding + (props.height - props.padding * 2) * (1 - value / max);
}

function shortLabel(label) {
  if (!label) return '';
  // 格式化后的时间是 "MM/DD HH:MM"
  // 只显示时间部分 HH:MM
  const parts = label.split(' ');
  return parts.length > 1 ? parts[1] : label;
}

function currency(v) {
  if (typeof v !== 'number') return '¥0';
  // 简化大额显示，避免文字过长被遮挡
  if (v >= 10000) {
    return `¥${(v / 10000).toFixed(1)}万`;
  } else if (v >= 1000) {
    return `¥${(v / 1000).toFixed(1)}k`;
  }
  return `¥${v.toFixed(0)}`;
}

// 工具提示使用完整金额显示
function fullCurrency(v) {
  if (typeof v !== 'number') return '¥0.00';
  return `¥${v.toFixed(2)}`;
}

function onEnter(p) {
  hover.value = p;
}
function onLeave() {
  hover.value = null;
}

const tooltipStyle = computed(() => {
  if (!hover.value) return {};
  const leftPct = (hover.value.x / props.width) * 100;
  const topPct = (hover.value.y / props.height) * 100;
  return { left: `${leftPct}%`, top: `${topPct}%` };
});
</script>

<style scoped>
.chart-wrapper {
  width: 100%;
  overflow: hidden;
  position: relative;
}
svg { width: 100%; height: auto; }
.line { fill: none; stroke: var(--accent-color); stroke-width: 2.5; }
.area { fill: url(#revenueGradient); stroke: none; }
.points circle { fill: var(--accent-color); stroke: var(--card-bg-color); stroke-width: 2; }
.points text { fill: var(--primary-text-color); font-size: var(--font-xs); }
.grid-lines line { stroke: var(--border-color-light); stroke-dasharray: 4 4; stroke-width: 1; }
.y-ticks text { fill: var(--primary-text-color); font-size: var(--font-xs); }
.chart-tooltip {
  position: absolute; transform: translate(-50%, -120%);
  background: var(--tooltip-bg); color: var(--text-white);
  padding: 0.5rem 0.75rem; border-radius: var(--radius-md); border: 1px solid var(--border-color-light);
  pointer-events: none; white-space: nowrap; box-shadow: var(--shadow-lg);
}
.tooltip-date { font-size: var(--font-sm); margin-bottom: 0.2rem; color: var(--secondary-text-color); }
.tooltip-value { font-size: var(--font-base); font-weight: 600; color: var(--primary-text-color); }
</style>