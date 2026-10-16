// Run in Chrome/Edge DevTools AFTER reproducing the delay. Recording starts
// automatically at the first edit; opening DevTools beforehand is unnecessary.
(() => {
  if (!window.__LAYMESH_WEB?.diagnostics) throw Error('请先打开 v009 首次编辑修复附件');
  const data=window.__LAYMESH_WEB.diagnostics();
  const url=URL.createObjectURL(new Blob([JSON.stringify(data,null,2)],{type:'application/json'}));
  const link=document.createElement('a');link.href=url;link.download='laymesh-first-edit-diagnostics.json';link.click();
  setTimeout(()=>URL.revokeObjectURL(url),1000);
  return data;
})();
