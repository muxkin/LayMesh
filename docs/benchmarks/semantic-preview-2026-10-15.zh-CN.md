# 语义增量预览：实现、验证与测量

日期：2026-10-15。工作区基线：`d82b74bc5f8ec6aab3221ecd70d80e15b3b3583a`，本次为本地修改与验证包，尚未发布。

本次已经完成共享编译会话、语义求值复用、绘图补丁和两个客户端接入，并生成独立附件与本地 VS Code 包。**这不是整份“完整属性级最小增量”计划已全部完成的声明**；保守依赖、元数据传输和连续新布局仍有后续工作，实时目标按下面的实测逐项判断。

## 实现

原生预览与离线 WASM 共用 `laymesh-core::session::CompileSession`。会话持有版本化文件、共享资源及字体数据库、语法节点身份、求值记录、几何与场景派生缓存、最近成功场景；提供 `apply_changes`、`evaluate`、`snapshot`、`dispose`。原有一次性编译入口使用同一求值内核的无缓存会话，作为对照路径。

解析按文本变化范围重新解析顶层语法区域，并对其中的表达式、函数体和列表元素匹配稳定身份。未变化子节点先匹配，避免在前方插入对象后把既有对象误认为新增。位置、参数和诊断来源单独重定位；无法可靠匹配时扩大解析范围。

求值记录以语法身份、调用路径、循环迭代位置为键，记录实际变量读取、闭包中的绑定、缺失绑定、资源读取及有序变量写入。复用前验证输入和执行上限；结果未变的子计算继续复用。函数、递归、条件、循环和模块走同一机制，没有把这些语法统一切回整页完整编译。

局部构造产生的对象作为独立贡献记录。回放时重新映射对象引用、锚点、公开名称和源位置；不可变参数与场景数据共享，后续修改按需复制。分支或循环退出后，旧贡献不进入新场景。外部可变副作用仍按原顺序执行，其内部安全的计算和几何可以复用。

几何缓存不包含颜色和位置；节点与布局缓存保留未变化的场景载荷。绘图转换直接借用既有命令生成属性或子节点结构补丁，避免先复制或序列化一整页再计算差异。纯平移保留局部路径，仅更新变换和动画布局；含有页面坐标绘制动画数据的子树会重新生成相应数据。字体变更及绘制动画绑定变化也会重新验证绘图上下文。

原生协议新增 `incremental-session-v1`，携带源码版本、文件变更和绘图基线；旧客户端继续使用完整结果。浏览器和 VS Code 的补丁应用保留未变化对象引用，渲染缓存复用路径、字形、图片和子树绘制内容。基线不匹配时恢复完整快照。渲染更新失败时，释放本次新建的原生句柄并丢弃相关缓存引用，防止恢复后复用已释放路径。作品文件格式、源文件、字体和原生路径不变。

两端默认输入合并窗口为 30 毫秒；VS Code 已配置值优先。组合输入期间暂停发布。原生编译在语义单元间检查取消；WASM 的文档规划和页面求值均可分时退出，让 Worker 接收新版本，再利用已完成的查询重试。保存、导出和整份播放经过最新文档屏障；错误或过期版本不能替换成功画面。

## 实现边界

- 当前采用从入口重放和验证依赖记录的方式，仍有控制流遍历与对象句柄管理开销，并非每次修改都只访问最终发生变化的节点。
- 对集合与部分对象读取采用整个值的依赖签名；共享样式按实际选择器查询记录匹配声明、作用域和顺序，新匹配规则也会使相关结果失效；父级输入仍有保守签名。文本、图像、图表及带复杂外部副作用的操作仍执行部分生成步骤，内部可复用计算继续命中缓存。尚不能把所有这些操作称为属性级最小增量更新。
- 检查信息、诊断和动画元数据按版本发送；当前没有对这些元数据全面做字段级补丁。普通局部绘图更新不发送整页路径。
- WASM 分时使用缓存后重试，没有保存解释器调用栈。为保证复杂首次编译最终完成，分片预算会从 200 毫秒逐步增加；因此不保证所有冷编译都能在固定的 200 毫秒内取消。
- 缓存预算约束可淘汰派生数据，活动源码、字体、语法树、成功场景与 WASM 线性内存容量需另计。WASM 保留容量不是活跃缓存占用，也不是进程总内存上限。
- VS Code 的整份放映准备目前重新建立后台页面工作进程；当前页保留会话的低延迟不能代表整份放映准备耗时，后者在 LCSS/laydoc 测量中单独报告。
- VS Code 组合输入通过委托内置 composition 命令实现，已验证实际窗口中的组合开始、结束和源码提交；与其他扩展争用同名命令的情况尚未完整覆盖。

## 显示完成信号修正

逐页即时截图发现，Worker 完成 GPU 提交后，OffscreenCanvas 的新画面仍可能尚未复制到页面。晚一点截图与独立编译结果逐像素一致，因此这是完成信号提前，并非场景数据永久错误。修正后，静态编辑先等待 GPU 完成及 Worker 的 rendering update，再跨过主页面的 rendering update，随后报告完成并更新概览缓存。主页面尺寸在交接期间变化时，继续绘制对应尺寸，避免提前恢复完成标记；连续动画仍采用原有非阻塞提交。静态源码预览保持动画时钟停止，重播动作显式启动时钟。后台不可见视图或未获得绘制机会的响应会标记未确认，不进入实时验收样本。

这一顺序也符合 [HTML 标准的 OffscreenCanvas 更新规则](https://html.spec.whatwg.org/multipage/canvas.html#the-offscreencanvas-interface)。`painted` 是经即时截图核对的应用层完成事件，仍不是显示器实际扫描输出的硬件测量。旧完成信号测得的时间偏低，不能作为“正确画面显示”的最终数据。最终样本要求 `presentationConfirmed=true`。

## LCSS 与 laydoc 新增验证

共享 LCSS 分别修改标题颜色和全部 `.strand` 曲线颜色；laydoc 分别改名、重排、新增和删除页面。新增与删除使用 100 次交替操作，分开统计，各保留 50 次样本，避免混合两种延迟分布。新增页引用已加载的第 2 页源码，每次使用不同的页面 id。当前页、页面目录和全部页面同步的延迟分别记录。浏览器的整份同步终点是全部页面编译完成；VS Code 的整份准备终点还包含放映资源的接收确认，两者不可直接当作同一指标比较。正式样本每个场景每个环境至少 50 次，试跑数据单独标记 `smoke`，不纳入验收。

仅改变文档名字、页面顺序或页面显示名称时，重新核对页面的实际文件读取、字体版本、源码路径及转场设置。保留客户端已确认的绘图基线，不重新计算未变化页面。目录依赖同时更新，确保后来修改新导入的文档模块仍能触发刷新；新增页面单独计算。

实际规则读取同时进入构造查询和布局节点缓存。测试要求修改无关标题规则时，曲线构造、曲线几何重建均为零，并保留复杂组的节点结果；新增后代选择器、媒体条件或继承样式仍与完整编译一致。缓存淘汰优先保留完整构造和组布局，在同类结果中优先保留所选页面，避免当前页的大量廉价中间值排挤其他页面的完整结果。不可变定义、内部映射、调用位置和资源共享存储，缓存按共享分配只计费一次，删除最后一个引用时释放相应预算。查询可借用未被几何及布局占用的额度，总预算保持不变；淘汰留出少量余量，减少每次插入都排序整个缓存的开销，预算仍约束被保留的数据。

中断回归发现，字面量 LCSS 特效构造内的让出信号曾被当作样式警告吞掉，偶尔会提交缺少阴影的结果。现在中断持续到事务退出，任何后续检查都返回同一中断，不写入未完成派生缓存。确定性检查点测试同时检查成功返回结果以及中断后的恢复结果；不能把丢失特效当作视觉容差。

## 测量方法

同一台 AMD Ryzen 7 8745H、16 逻辑线程、约 20.3 GiB 内存的 Linux 机器。浏览器使用 Playwright 的 headless 模式、离线 `file://`，禁止网络访问，视口 1600×1000；实际 VS Code 使用独立用户目录、原生编译服务与实际 Webview，视口 1440×1000、DPR 2。版本分别为 Chromium 151.0.7922.34、Firefox 153.0、WebKit 26.5 和 VS Code 1.113.0。浏览器和 VS Code 的图形环境不同，分别报告，不能把它们当作纯编译器速度比较，也不能将 headless 结果直接外推到日常桌面浏览器。

选择第 2 页和含 1,536 条曲线的第 6 页，分别改变背景颜色、局部位置、标题和矩形尺寸。浏览器每项排除 2 次预热后测量 50 次不同值；VS Code 每项排除 1 次预热后测量 50 次不同值。另保留交替修改与撤销的诊断结果，但验收主表采用不同值，避免仅测到已有布局缓存的反复命中。指标包含输入合并窗口，终点为正确版本的画面完成绘制。完整原始记录包含编译、绘图准备、栅格化、实际复用计数与内存指标。

浏览器响应中的普通 `compileMetrics` 属于最后一次 `evaluate` 尝试；发生 `E_YIELD` 时，不能仅凭这组计数断言整个编辑没有重算。复用校验另用测试插桩累计每次成功或中断尝试的指标，插桩只读取指标，不改变 WASM、源码或求值逻辑，其耗时不进入性能验收。

测量期间未并行运行本任务的构建或其他性能测试；保留用户桌面及其他进程。自动化调用的往返时间另存，主指标使用编辑器和画面完成事件，不把 Playwright 轮询等待算作应用延迟。VS Code 正式计时关闭周期性 CDP DOM 诊断，只在开始时设置视口；需要诊断时可单独设置 `LAYMESH_PERFORMANCE_DEBUG=1`，其结果不进入汇总。

浏览器使用原附件嵌入的字体字节；VS Code 基准从同一项目提取源码和资源，使用本机已安装的具名字体。这一字体环境差异保留在记录中，不能据此宣称浏览器与 VS Code 像素完全相同。

<!-- FINAL_MEASUREMENTS_START -->

共 2,800 次正式编辑样本；每个场景 50 次，已排除预热。新增与删除分别统计，各 50 次。中位数取中间两值的平均，P95 使用最近秩。

### 局部 `.lay` 修改

单位为毫秒，表中数值为 **中位数 / P95**。目标为中位数 ≤100 毫秒且 P95 ≤200 毫秒。

| 环境 | 页面 | 颜色 | 位置 | 文字 | 尺寸 |
|---|---|---:|---:|---:|---:|
| Chromium | 第 2 页 | 111.4 / 112.9 | 112.1 / 113.4 | 112.8 / 113.7 | 112.4 / 113.4 |
| Chromium | 第 6 页，1,536 条曲线 | 145.5 / 162.2 | 177.0 / 178.6 | 159.7 / 161.9 | 177.0 / 192.7 |
| Firefox | 第 2 页 | 279.0 / 281.0 | 279.0 / 281.0 | 280.0 / 297.0 | 279.0 / 285.0 |
| Firefox | 第 6 页，1,536 条曲线 | 453.5 / 662.0 | 729.0 / 744.0 | 462.0 / 680.0 | 729.0 / 746.0 |
| WebKit | 第 2 页 | 93.0 / 95.0 | 94.0 / 95.0 | 93.0 / 94.0 | 93.0 / 109.0 |
| WebKit | 第 6 页，1,536 条曲线 | 125.0 / 141.0 | 157.0 / 172.0 | 139.0 / 142.0 | 157.0 / 161.0 |
| VS Code | 第 2 页 | 104.0 / 111.0 | 105.0 / 112.0 | 113.5 / 123.0 | 113.0 / 125.0 |
| VS Code | 第 6 页，1,536 条曲线 | 168.5 / 182.0 | 204.5 / 227.0 | 174.0 / 192.0 | 204.5 / 219.0 |

32 个场景中，4 个同时达到两项延迟目标。

### LCSS 与 laydoc 修改

单位为毫秒，均为 **中位数 / P95**。当前页固定为第 6 页。浏览器目录终点是 DOM 更新，VS Code 目录终点是宿主收到新目录；整份同步分别是全部页面编译完成、整份放映及资源确认。

| 环境 | 修改 | 当前页显示 | 目录更新或接收 | 整份同步 |
|---|---|---:|---:|---:|
| Chromium | LCSS 标题样式 | 158.1 / 166.6 | 90.1 / 96.3 | 8819.4 / 8975.8 |
| Chromium | LCSS 全部曲线样式 | 2526.3 / 2569.1 | 2412.1 / 2445.8 | 11468.5 / 11590.7 |
| Chromium | laydoc 改名 | 33.3 / 34.6 | 35.2 / 36.6 | 35.3 / 36.8 |
| Chromium | laydoc 重排 | 34.0 / 35.1 | 35.7 / 37.0 | 35.9 / 37.1 |
| Chromium | laydoc 新增页面 | 32.8 / 33.4 | 34.6 / 35.7 | 212.5 / 218.9 |
| Chromium | laydoc 删除页面 | 46.2 / 48.8 | 47.9 / 50.1 | 48.0 / 50.3 |
| Firefox | LCSS 标题样式 | 471.0 / 486.0 | 291.0 / 300.0 | 26826.0 / 30620.0 |
| Firefox | LCSS 全部曲线样式 | 9993.5 / 19878.0 | 9701.0 / 9784.0 | 45673.5 / 55170.0 |
| Firefox | laydoc 改名 | 36.0 / 38.0 | 37.0 / 39.0 | 37.0 / 39.0 |
| Firefox | laydoc 重排 | 38.0 / 40.0 | 39.0 / 42.0 | 39.0 / 43.0 |
| Firefox | laydoc 新增页面 | 36.0 / 37.0 | 37.0 / 40.0 | 1114.0 / 1129.0 |
| Firefox | laydoc 删除页面 | 63.0 / 67.0 | 65.0 / 68.0 | 65.0 / 68.0 |
| WebKit | LCSS 标题样式 | 144.0 / 158.0 | 92.0 / 110.0 | 8654.0 / 8855.0 |
| WebKit | LCSS 全部曲线样式 | 2520.0 / 2550.0 | 2419.5 / 2453.0 | 11478.0 / 11741.0 |
| WebKit | laydoc 改名 | 34.0 / 35.0 | 36.0 / 38.0 | 36.0 / 38.0 |
| WebKit | laydoc 重排 | 35.0 / 36.0 | 37.0 / 38.0 | 37.0 / 38.0 |
| WebKit | laydoc 新增页面 | 34.0 / 34.0 | 35.0 / 37.0 | 266.5 / 276.0 |
| WebKit | laydoc 删除页面 | 46.0 / 48.0 | 48.0 / 50.0 | 48.0 / 51.0 |
| VS Code | LCSS 标题样式 | 197.5 / 213.0 | 111.0 / 135.0 | 4887.5 / 4956.0 |
| VS Code | LCSS 全部曲线样式 | 1865.5 / 1927.0 | 1722.5 / 1782.0 | 6554.5 / 6644.0 |
| VS Code | laydoc 改名 | 193.5 / 205.0 | 109.0 / 115.0 | 4868.0 / 4894.0 |
| VS Code | laydoc 重排 | 191.0 / 203.0 | 106.0 / 118.0 | 4902.5 / 5561.0 |
| VS Code | laydoc 新增页面 | 195.0 / 210.0 | 110.5 / 124.0 | 4927.0 / 4980.0 |
| VS Code | laydoc 删除页面 | 197.0 / 207.0 | 109.5 / 115.0 | 4892.0 / 4956.0 |

### 浏览器内存记录

这些是连续 400 次局部编辑中的峰值。可淘汰缓存估计值与编译器 WASM 保留容量含义不同，均不是浏览器进程总内存。

| 环境 | 可淘汰缓存峰值 MiB | 编译器 WASM 保留容量峰值 MiB | 最后 100 次容量稳定 |
|---|---:|---:|---|
| Chromium | 219.8 | 710.1 | 是 |
| Firefox | 219.8 | 709.9 | 是 |
| WebKit | 219.8 | 710.3 | 是 |

原始样本与汇总见本报告同名数据目录中的 `summary.json`、`browser`、`browser-document`、`vscode` 和 `vscode-document`。


### 结果判断

局部 `.lay` 修改已能复用复杂路径，但本次 **32 个场景只有 4 个达到中位数 ≤100 毫秒且 P95 ≤200 毫秒**，均为 WebKit 第 2 页。Chromium 第 6 页约 145～177 毫秒，VS Code 约 169～205 毫秒；Firefox 的当前测试环境明显更慢。不能据此宣称已经实现全面实时预览。

LCSS 标题样式与全部曲线样式的差别很大。Chromium 当前页分别约 158 毫秒和 2.53 秒；Firefox 全部曲线样式的 P95 达到 19.88 秒。几何复用不等于整个样式求值、对象验证、绘图补丁和显示过程都没有成本。共享样式的整份同步仍需数秒到数十秒，属于尚未达到预期的路径。

浏览器 laydoc 改名和重排当前页约 33～38 毫秒，新增页面约 33～36 毫秒，但整份同步还需完成新增页的计算。VS Code 的对应当前页约 191～197 毫秒，整份准备约 4.9 秒；其整份放映仍会重建后台页面工作进程，持续复用这组会话尚未接入。两端的目录、当前页和整份完成终点必须分开看。

这里没有把最初的 1.84 秒单页 WASM 编译和新的端到端显示时间直接计算成百分比：两者终点、浏览器环境和执行路径不同。原附件与新附件的同环境视觉一致性有独立验证；性能表用于判断新实现的绝对延迟目标。

### 全部尝试的复用计数

以下是测试插桩的单次编辑校验，累计成功及中断尝试；不纳入端到端性能统计。尝试次数随分时切分变化。

| 环境 | 修改 | 求值尝试 | `path` 构造次数 | 几何重建次数 |
|---|---|---:|---:|---:|
| Chromium | 局部颜色 | 1 | 0 | 0 |
| Chromium | 局部位置 | 1 | 0 | 0 |
| Chromium | 局部标题 | 1 | 0 | 0 |
| Chromium | 矩形尺寸 | 1 | 0 | 1 |
| Chromium | LCSS 标题样式 | 1 | 0 | 0 |
| Chromium | LCSS 全部曲线样式 | 4 | 2696 | 0 |
| Firefox | 局部颜色 | 2 | 0 | 0 |
| Firefox | 局部位置 | 2 | 0 | 0 |
| Firefox | 局部标题 | 2 | 0 | 0 |
| Firefox | 矩形尺寸 | 2 | 0 | 1 |
| Firefox | LCSS 标题样式 | 2 | 0 | 0 |
| Firefox | LCSS 全部曲线样式 | 6 | 2462 | 0 |
| WebKit | 局部颜色 | 1 | 0 | 0 |
| WebKit | 局部位置 | 1 | 0 | 0 |
| WebKit | 局部标题 | 1 | 0 | 0 |
| WebKit | 矩形尺寸 | 1 | 0 | 1 |
| WebKit | LCSS 标题样式 | 1 | 0 | 0 |
| WebKit | LCSS 全部曲线样式 | 4 | 2795 | 0 |

无关标题修改没有执行曲线构造；矩形尺寸修改只重建 1 份几何。全曲线 LCSS 改色复用了全部路径几何，但样式依赖失效与分时重试仍重复执行部分对象构造。这说明几何复用已经验证，同时也证实该路径尚不是完整属性级最小更新。详见 [所有尝试的原始计数](semantic-preview-2026-10-15/all-attempt-reuse.json)。

<!-- FINAL_MEASUREMENTS_END -->

## 验证范围

核心对照测试将修改、撤销、错误和恢复序列与无缓存编译比较，检查场景、诊断、字体及来源信息；覆盖闭包写入、递归、循环增减、分支、集合、模块、共享样式、字体与资源替换、对象增删和锚点。强制低预算的连续编辑验证缓存淘汰；事务和取消测试验证最近成功场景不被失败版本覆盖。

绘图补丁测试检查属性修改、插入删除重排、基线拒绝、资源变化、裁剪与变换；组平移测试同时比较静态像素及动画采样，并断言不包含路径替换。实际原生协议检查客户端协商、变更集、补丁对照、错误恢复与取消。

六页分别改色、修改标题和尺寸后，将保留会话完成事件后的画布位图与独立编译会话逐像素比较，并比较规范化绘图命令。保留局部路径与冷编译的页面变换再逆变换可能产生浮点尾数差异；数值绝对差上限为 `1e-10`，最终 54 项对照的实际最大差为 `1.14e-13`；结构、非数值内容和最终位图仍严格比较。Firefox 的 Skia CPU 在相同源数据首次绘制和复用原生路径绘制之间出现过 9 个像素、最大 5/255 的抗锯齿差异；对此只重绘独立对照画面，不重新获取被测的即时画面，并记录是否需要此对照。最终共 6 项使用原样重绘的独立对照，分别为 Firefox 和 WebKit 首页的三类编辑，其余 48 项首次画面直接一致。常规功能回归同时在完成信号后立即读取实际画面背景像素，禁止添加延时来掩盖旧画面。

三种浏览器的功能测试覆盖连续输入、组合输入、编译中切页、共享模块与样式、图片缺失和恢复、文档结构、过期响应、保存和导出屏障。六页附件与原附件逐页比较规范化绘图数据，全部六页动画在 0、3、6.5 秒采样比较像素哈希，三种浏览器共 54 组新旧附件对照通过；实际下载、重开可编辑版及播放版另行验证。此处的组合输入验证使用真实编辑器事件，未覆盖操作系统各个中文输入法的候选窗行为。

VS Code 功能测试使用实际窗口、原生编译进程与 Webview，覆盖未保存的导入模块、文件资源传输、错误跳转、组合输入、快速编辑、旧响应、超时重启、重载、多预览窗口、文档切页与重排，以及 PDF、MP4 导出。

最终候选的完整 Rust 回归：4 个包、73 个测试套件，共 **641 项测试通过、0 项失败**，其中会话对照测试 26 项；另有 1 项手动微基准按原设置忽略。工作区所有 targets 的编译检查通过；最终源码格式检查通过。JavaScript 调度、补丁不可变性和渲染句柄错误恢复检查通过。文档示例重新生成并校验 **231 个来源**，工作区媒体差异仅为清单中的引擎指纹。

平移回归还核对匿名路径子节点的动画布局元数据：移动外层形状时同步更新这些复制的提示，保持路径与局部变换，防止残留旧位置。

原生范围测试包含 128 条曲线及独立的 32 个椭圆：公式修改产生 128 次曲线构造、128 次几何重建、0 次椭圆构造，单次原生请求约 73.3 毫秒；循环 128→129 只产生 1 次曲线构造和 1 次几何重建，约 11.1 毫秒。该数据是单次范围验证，包含原生响应，不是浏览器端到端分位数。对照的完整编译不计入该时间。详见 [范围验证数据](semantic-preview-2026-10-15/native-protocol-delivery.json)。

## 复跑

在仓库根目录运行。`PLAYWRIGHT_MODULE` 指向已安装的 Playwright 包；浏览器引擎需事先安装。HTML 测试不启动网页服务。

```sh
cargo test -p laymesh-core --test compile_session
cargo test -p laymesh-render --test incremental_lowering --test display_updates --test animation
node scripts/test-display-update.mjs
node scripts/test-skia-update-recovery.mjs
node scripts/test-web-preview-compiler.mjs
python3 scripts/test-preview-semantic-session.py --binary target/release/laymesh --output /tmp/laymesh-protocol.json

python3 scripts/build-web-export.py
cargo build --release --locked -p laymesh-cli
python3 scripts/build-editors.py --binary target/release/laymesh --extension-dir /tmp/laymesh-semantic-extension
node scripts/refresh-web-runtime.mjs ORIGINAL.zip.html NEW.zip.html

PLAYWRIGHT_MODULE=/path/to/playwright node scripts/test-web-semantic-performance.cjs NEW.zip.html /tmp/laymesh-validation/browser 50
PLAYWRIGHT_MODULE=/path/to/playwright node scripts/test-web-document-performance.cjs NEW.zip.html /tmp/laymesh-validation/browser-document 50
PLAYWRIGHT_MODULE=/path/to/playwright node scripts/test-web-incremental-preview.cjs /tmp/laymesh-web-functional
PLAYWRIGHT_MODULE=/path/to/playwright node scripts/test-web-semantic-roundtrip.cjs ORIGINAL.zip.html NEW.zip.html /tmp/laymesh-roundtrip
PLAYWRIGHT_MODULE=/path/to/playwright node scripts/test-web-semantic-edit-oracle.cjs NEW.zip.html /tmp/laymesh-edit-oracle
PLAYWRIGHT_MODULE=/path/to/playwright node scripts/test-web-semantic-reuse.cjs NEW.zip.html /tmp/laymesh-reuse

node scripts/prepare-semantic-preview-fixture.mjs ORIGINAL.zip.html /tmp/laymesh-semantic-fixture
python3 scripts/test-vscode-semantic-performance.py --fixture /tmp/laymesh-semantic-fixture --output /tmp/laymesh-validation/vscode --binary target/release/laymesh --extension-dir /tmp/laymesh-semantic-extension
python3 scripts/test-vscode-document-performance.py --fixture /tmp/laymesh-semantic-fixture --output /tmp/laymesh-validation/vscode-document --binary target/release/laymesh --extension-dir /tmp/laymesh-semantic-extension
python3 scripts/test-vscode-preview.py --output /tmp/laymesh-vscode-functional --extension-dir /tmp/laymesh-semantic-extension
python3 scripts/summarize-semantic-preview-performance.py --input /tmp/laymesh-validation --output /tmp/laymesh-summary
```

`NEW.zip.html` 和新夹具目录必须尚不存在，避免覆盖原件。生成附件的工具逐项核对全部非运行时作品内容，保持源码、字体、样式、路径和动画资产一致。

## 交付文件

- 独立可编辑 HTML（本地产物：`video/04-LCSS与动画/exports/semantic-preview-v002/04_LCSS_离线可编辑_v005_语义增量预览.zip.html`）：沿用原附件的 31 个非运行时内容条目，仅刷新运行时；原附件保留。
- VS Code Linux x64 本地验证包（`video/04-LCSS与动画/exports/semantic-preview-v002/laymesh-0.6.4-semantic-preview-linux-x64.vsix`）：包含本次原生编译器与 Webview。未安装到日常用户配置，未发布到 Marketplace。
- [原始测量与汇总](semantic-preview-2026-10-15/summary.json)、[测量环境](semantic-preview-2026-10-15/environment.json)、[产物与哈希](semantic-preview-2026-10-15/artifacts.json)。

HTML SHA-256：`2a13ed41f6fb65e2144b2f4f6bb04c55c08299ecc193d57a8390828803709520`。

VSIX SHA-256：`9d20a0f7e7dcbda75eaaa0557ef9cf320bec2fa85b6554a27eb0982186ab6985`。

本次没有提交或推送。已有裁剪修改及其测试、CI 条目保留。正式计时后只统一了 Rust 源码格式并补充测试与报告，交付运行时未更换；测量始终对应上述 HTML 和记录中的原生二进制哈希。
