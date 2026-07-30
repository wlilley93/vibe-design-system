'use strict';

// An assembly ASSEMBLES. It calls the same block functions the manifest could call
// directly rather than restating their markup, so a fix to objecttable.js reaches
// this layout too. Restating it would be a second source of truth for the same
// component, which is the defect this whole factory is arranged to avoid.
const objecttable = require('./objecttable.js');
const objectview = require('./objectview.js');
const inspector = require('./inspector.js');
const facetstrip = require('./facetstrip.js');

// masterdetail-1: list beside detail. Two panes, the classic object workspace.
function twoPane(content) {
  const facets = content.facets ? facetstrip['facetstrip-1'](content.facets) : '';
  const master = objecttable['objecttable-2'](content.master);
  const detail = objectview['objectview-2'](content.detail);
  return `<div class="md md--two">
  ${facets}
  <div class="md__panes">
    <div class="md__master">${master}</div>
    <div class="md__detail">${detail}</div>
  </div>
</div>`;
}

// masterdetail-2: list, detail, and an inspector rail. Three panes, for a surface
// where the object's properties or its audit trail need to stay visible.
function threePane(content) {
  const facets = content.facets ? facetstrip['facetstrip-1'](content.facets) : '';
  const master = objecttable['objecttable-2'](content.master);
  const detail = objectview['objectview-2'](content.detail);
  const rail = inspector[content.inspectorVariant === 'activity' ? 'inspector-2' : 'inspector-1'](content.inspector);
  return `<div class="md md--three">
  ${facets}
  <div class="md__panes">
    <div class="md__master">${master}</div>
    <div class="md__detail">${detail}</div>
    <div class="md__rail">${rail}</div>
  </div>
</div>`;
}

module.exports = {
  'masterdetail-1': twoPane,
  'masterdetail-2': threePane,
};
