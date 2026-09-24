<template>
  <div class="fixture">stylelint 违规样本（由 src/gates.spec.ts 断言）</div>
</template>

<style scoped>
.a {
  /* expect: color-no-hex */
  color: #fff;
}

.b {
  /* expect: color-named */
  color: red;
}

.c {
  /* expect: function-disallowed-list */
  background: rgba(0, 0, 0, 0.1);
}

.d {
  /* expect: declaration-property-value-allowed-list */
  font-size: 14px;
  /* expect: declaration-property-value-allowed-list */
  font-weight: 700;
  /* expect: declaration-property-value-allowed-list */
  border-radius: 6px;
  /* expect: declaration-property-value-allowed-list */
  box-shadow: 0 1px 2px var(--shadow-color);
  /* expect: declaration-property-value-allowed-list */
  gap: 10px;
  /* expect: declaration-property-value-allowed-list */
  padding: 5px 8px;
  /* expect: declaration-property-value-allowed-list */
  margin-top: 3px;
  /* expect: declaration-property-value-allowed-list */
  max-width: 900px;
}

.g {
  /* expect: declaration-property-value-allowed-list */
  font: 14px/1.4 sans-serif;
}

/* 以下是终审补的绕过写法，每条都必须被拦下 */
.h {
  /* expect: function-disallowed-list */
  color: oklch(0.5 0.2 30);
  /* expect: function-disallowed-list */
  border-color: color(srgb 1 0 0);
  /* expect: declaration-property-value-disallowed-list */
  fill: CanvasText;
  /* expect: declaration-property-value-disallowed-list */
  border: 1px solid highlight;
  /* expect: property-disallowed-list */
  --space-huge: 13px;
  /* expect: declaration-property-value-allowed-list */
  -webkit-box-shadow: 0 0 3px var(--shadow-color);
  /* expect: declaration-property-value-allowed-list */
  grid-gap: 10px;
  /* expect: declaration-property-value-allowed-list */
  border-start-start-radius: 6px;
  /* expect: declaration-property-value-allowed-list */
  max-width: 60rem;
  /* expect: declaration-property-value-allowed-list */
  max-width: min(1280px, 90%);
}

/* expect: at-rule-disallowed-list */
@custom-media --mine (max-width: 700px);

/* expect: at-rule-disallowed-list */
@container (width < 700px) {
  .i {
    display: none;
  }
}

/* expect: media-feature-name-disallowed-list */
@media (max-width: 768px) {
  .e {
    display: none;
  }
}

/* expect: comment-no-empty */
/* */

/* expect: (none) */
@media (--phone) {
  .f {
    padding: var(--space-sm) 0;
    gap: calc(var(--space-sm) * 2);
  }
}

/* expect: (none) */
.legal {
  color: var(--primary-text-color);
  font-size: var(--font-base);
  font-weight: var(--weight-bold);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-sm);
  gap: var(--space-sm);
  padding: 0 var(--space-md);
  margin: auto;
  max-width: var(--page-content);
  background: color-mix(in srgb, var(--accent-color) 50%, transparent);
  border-color: var(--highlight-color);
  --pg-media-pad: 133%;
}
.legal-5 {
  max-width: 35rem;
}

/* expect: (none) */
.legal-2 {
  max-width: min(100%, 65vh);
}
.legal-3 {
  max-width: 320px;
}
.legal-4 {
  max-width: 8em;
}
</style>
