# 🔧 运维手册

## 日常运维

### 1. 服务启动
```bash
# 启动后端
cd Rust-Blockchain-Secure-Wallet
./target/release/blockchain-wallet-server

# 启动前端（开发）
cd "Wallet front-end/blockchain-wallet-ui"
npm start
```

---

### 2. 服务状态检查
```bash
# 检查后端健康
curl http://127.0.0.1:8080/api/health

# 检查进程
ps aux | grep blockchain-wallet-server

# 检查端口
netstat -tulpn | grep 8080
```

---

### 3. 日志查看
```bash
# 实时日志
tail -f /var/log/wallet-backend.log

# 错误日志
grep ERROR /var/log/wallet-backend.log

# 最近100行
tail -100 /var/log/wallet-backend.log
```

---

## 监控指标

### 关键指标

| 指标 | 正常值 | 告警阈值 |
|------|--------|---------|
| CPU使用率 | <30% | >80% |
| 内存使用 | <1GB | >1.5GB |
| 响应时间 | <100ms | >500ms |
| 错误率 | <1% | >5% |
| 并发连接 | <100 | >500 |

---

### 监控命令
```bash
# CPU和内存
top -p $(pgrep blockchain-wallet)

# 网络连接
netstat -an | grep 8080 | wc -l

# 请求速率
tail -f access.log | pv -l -i 1 > /dev/null
```

---

## 备份策略

### 1. 数据备份
```bash
# 备份钱包数据（如使用SQLite）
cp wallet_data.db wallet_data.db.backup.$(date +%Y%m%d)

# 定期备份
0 2 * * * /opt/scripts/backup-wallet-data.sh
```

### 2. 配置备份
```bash
# 备份配置文件
tar -czf config-backup-$(date +%Y%m%d).tar.gz config/
```

---

## 故障处理

### 场景1：后端崩溃
```bash
# 检查日志
tail -100 /var/log/wallet-backend.log

# 重启服务
systemctl restart blockchain-wallet

# 验证
curl http://127.0.0.1:8080/api/health
```

---

### 场景2：性能下降
```bash
# 检查CPU和内存
top

# 检查连接数
netstat -an | grep 8080 | wc -l

# 检查慢查询
grep "took.*ms" /var/log/wallet-backend.log
```

---

### 场景3：磁盘满
```bash
# 检查磁盘
df -h

# 清理日志
find /var/log -name "*.log" -mtime +30 -delete

# 压缩旧日志
gzip /var/log/wallet-backend.log.old
```

---

## 安全检查清单

### 每日检查
- [ ] 检查异常登录
- [ ] 检查错误日志
- [ ] 检查磁盘空间
- [ ] 检查服务状态

### 每周检查
- [ ] 审查访问日志
- [ ] 检查安全漏洞
- [ ] 更新依赖包
- [ ] 备份验证

### 每月检查
- [ ] 性能分析
- [ ] 容量规划
- [ ] 安全审计
- [ ] 灾备演练

---

## 应急预案

### 严重故障
1. 通知用户（维护公告）
2. 切换到备用服务器
3. 排查问题
4. 修复并测试
5. 恢复服务
6. 事后分析

### 安全事件
1. 立即隔离受影响系统
2. 分析攻击向量
3. 修复漏洞
4. 通知受影响用户
5. 提交安全报告

---

## 性能优化

### 后端优化
```rust
// 启用缓存
// 增加连接池
// 优化数据库查询
// 使用异步I/O
```

### 前端优化
```bash
# 代码分割
npm run build

# 启用Gzip
# 使用CDN
# 图片压缩
```

---

## 更新流程

### 1. 准备
- 备份数据
- 通知用户
- 准备回滚方案

### 2. 更新
```bash
# 拉取新代码
git pull

# 编译
cargo build --release

# 停止服务
systemctl stop blockchain-wallet

# 替换二进制
cp target/release/blockchain-wallet-server /usr/local/bin/

# 启动服务
systemctl start blockchain-wallet
```

### 3. 验证
- 检查健康状态
- 运行冒烟测试
- 监控错误日志

---

**版本**: v1.0  
**维护**: 运维团队

