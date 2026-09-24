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
}
</style>
