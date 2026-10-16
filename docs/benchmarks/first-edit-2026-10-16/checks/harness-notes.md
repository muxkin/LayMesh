# 测试启动顺序修正

追加的 animation-edit 阶段第一次运行，在编辑器尚未挂载时访问 files，报 TypeError。该次在修改作品前退出（6.3 秒），没有产生性能样本，也未更改交付物。测试脚本补充等待 __LAYMESH_WEB.editor 后重跑，三浏览器全部通过。validation-status.json 保留失败和成功两条记录；animation-edit.log 是成功重跑的输出。正式 300 次编辑统计来自此前已通过的 first-edit 阶段。
