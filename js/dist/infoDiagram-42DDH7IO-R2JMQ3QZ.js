import {
  parse
} from "./chunk-LGUKRCLM.js";
import {
  selectSvgElement
} from "./chunk-4YDCQRHO.js";
import "./chunk-ZQDQZO2Y.js";
import "./chunk-BHGYWDGT.js";
import "./chunk-D2D2P73Z.js";
import "./chunk-AA5A7ZBG.js";
import "./chunk-23HQIFW5.js";
import "./chunk-FDMIMDSF.js";
import "./chunk-HXFIRRKG.js";
import "./chunk-NMZGTPTN.js";
import {
  configureSvgSize
} from "./chunk-OOB32QPG.js";
import {
  __name,
  log
} from "./chunk-VS6SMCEU.js";
import "./chunk-IHZMWFST.js";
import "./chunk-XFNOBYEQ.js";
import "./chunk-T3B4F33H.js";
import "./chunk-55WCR2MV.js";
import "./chunk-5MASLJB6.js";
import "./chunk-WPLOTFGW.js";

// node_modules/.pnpm/mermaid@11.14.0/node_modules/mermaid/dist/chunks/mermaid.core/infoDiagram-42DDH7IO.mjs
var parser = {
  parse: /* @__PURE__ */ __name(async (input) => {
    const ast = await parse("info", input);
    log.debug(ast);
  }, "parse")
};
var DEFAULT_INFO_DB = {
  version: "11.14.0" + (true ? "" : "-tiny")
};
var getVersion = /* @__PURE__ */ __name(() => DEFAULT_INFO_DB.version, "getVersion");
var db = {
  getVersion
};
var draw = /* @__PURE__ */ __name((text, id, version) => {
  log.debug("rendering info diagram\n" + text);
  const svg = selectSvgElement(id);
  configureSvgSize(svg, 100, 400, true);
  const group = svg.append("g");
  group.append("text").attr("x", 100).attr("y", 40).attr("class", "version").attr("font-size", 32).style("text-anchor", "middle").text(`v${version}`);
}, "draw");
var renderer = { draw };
var diagram = {
  parser,
  db,
  renderer
};
export {
  diagram
};
