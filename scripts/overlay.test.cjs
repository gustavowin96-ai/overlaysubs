const fs = require('node:fs');
const vm = require('node:vm');
const assert = require('node:assert/strict');
const { test } = require('node:test');
const source = fs.readFileSync('ui/overlay.js', 'utf8');
function context(mode) {
  const ctx = vm.createContext({});
  vm.runInContext(`let displayMode=${JSON.stringify(mode)}, cfg={showSource:false};
    const caps=new Map();
    const hasWords=s=>/[\\p{L}\\p{N}]/u.test(s||'');
    const esc=s=>String(s??'').replace(/[&<>]/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;'}[c]));
    ${source.slice(source.indexOf('const captionHtml'), source.indexOf('/** Two sample'))}
    ${source.slice(source.indexOf('const readable'), source.indexOf('function draw()'))}
    ${source.slice(source.indexOf('function merge('), source.indexOf('listen("caption"'))}
  `, ctx);
  return ctx;
}
test('English appears without translation and bilingual keeps English above Portuguese', () => {
  const c=context('english');
  assert.match(vm.runInContext('captionHtml({source:"Hello",translation:"",stage:"partial"})',c),/Hello/);
  assert.equal(vm.runInContext('isReadable({source:"Hello",translation:""})',c),true);
  vm.runInContext('displayMode="bilingual"',c);
  const html=vm.runInContext('captionHtml({source:"Hello",translation:"Olá",stage:"final"})',c);
  assert.ok(html.indexOf('Hello')<html.indexOf('Olá'));
  vm.runInContext('displayMode="portuguese"',c);
  assert.doesNotMatch(vm.runInContext('captionHtml({source:"Hello",translation:"Olá"})',c),/Hello/);
});
test('late translations retain sentence order and escape recognized HTML', () => {
  const c=context('bilingual');
  vm.runInContext('caps.set(2,{id:2,source:"second"});caps.set(1,{id:1,source:"first",translation:"primeiro"})',c);
  assert.equal(vm.runInContext('readable().map(c=>c.id).join(",")',c),'1,2');
  assert.match(vm.runInContext('captionHtml({source:"<script>",translation:""})',c),/&lt;script&gt;/);
});
test('translation updates replace only matching caption and preserve draft text', () => {
  const c=context('bilingual');
  assert.equal(vm.runInContext('merge({translation:"Olá"},{stage:"streaming",translation:"O"}).translation',c),'Olá');
  assert.equal(vm.runInContext('merge({translation:"Olá"},{stage:"final",translation:"Oi"}).translation',c),'Oi');
});
