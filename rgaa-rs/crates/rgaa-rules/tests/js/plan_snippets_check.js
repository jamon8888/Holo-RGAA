// Manual check of src/plan_snippets.rs against fixture pages. Needs jsdom (npm i jsdom); run: node plan_snippets_check.js
// Expect: every snippet FAILs on the 'bad' page and passes on the 'good' one, except 4.5 (the good fixture has no audio description).
const {JSDOM}=require('jsdom');
const fs=require('fs');
const src=fs.readFileSync('__dirname+'/../src/plan_snippets.rs'','utf8');
const sn={};for(const m of src.matchAll(/"(\d+\.\d+)",\s*r#"([\s\S]*?)"#,?\s*\),/g))sn[m[1]]=m[2];
const bad=`<html lang=fr><head><style>a:focus{outline:none} .x{color:red}</style></head><body>
<font>x</font><center>y</center>
<p dir=sideways>z</p>
<p>« Ceci est une longue citation qui devrait être dans un blockquote vraiment. »</p>
<table><tr><td colspan=2>a</td></tr></table>
<table role=presentation><tr><th>h</th></tr></table>
<video src=a.mp4></video><object data=x.swf></object>
<select onchange="this.form.submit()"><option>a</option></select>
<div style="color:red">t</div>
<fieldset><input></fieldset>
<a href=x target=_blank>lien</a>
<span>Super :) merci</span>
<blockquote>ok</blockquote></body></html>`;
const good=`<!DOCTYPE html><html lang=fr><head><style>a:focus{outline:2px solid red} .x{color:red;background:#fff}</style></head><body>
<p>texte</p><blockquote>« Ceci est une longue citation qui devrait être dans un blockquote vraiment. »</blockquote>
<table><caption>c</caption><tr><td colspan=2>a</td></tr></table>
<figure><video src=a.mp4 controls title="v"><track kind=captions></video><figcaption>Transcription</figcaption></figure>
<object data=x.swf>Alternative texte</object>
<fieldset><legend>L</legend><input></fieldset>
<a href=x target=_blank>lien (nouvelle fenêtre)</a><div style="color:red;background:#fff">t</div>
</body></html>`;
function run(html){const w=new JSDOM(html,{runScripts:'outside-only'}).window;const o={};for(const [id,c] of Object.entries(sn)){try{const r=JSON.parse(w.eval(c));o[id]=r.pass?'ok':'FAIL('+r.nodes+')'}catch(e){o[id]='ERR '+e.message}}return o}
console.log(Object.keys(sn).length+' snippets');
console.log('bad ',JSON.stringify(run(bad)));
console.log('good',JSON.stringify(run(good)));
