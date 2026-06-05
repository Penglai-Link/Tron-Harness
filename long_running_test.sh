#!/bin/bash
# 长时间运行测试脚本
# 模拟一个需要较长时间完成的任务

echo "=========================================="
echo "  长时间脚本运行测试"
echo "  开始时间: $(date '+%Y-%m-%d %H:%M:%S')"
echo "=========================================="
echo ""

TOTAL_STEPS=10
SLEEP_INTERVAL=2

for i in $(seq 1 $TOTAL_STEPS); do
    echo "[$(date '+%H:%M:%S')] 步骤 $i/$TOTAL_STEPS - 处理中..."

    # 模拟一些工作（使用sleep代替复杂计算，避免bash算术溢出）
    sleep 1

    # 模拟输出一些工作成果
    echo "[$(date '+%H:%M:%S')] 步骤 $i/$TOTAL_STEPS - 完成 ✓"

    if [ $i -lt $TOTAL_STEPS ]; then
        echo "[$(date '+%H:%M:%S')] 等待 ${SLEEP_INTERVAL} 秒后继续..."
        sleep $SLEEP_INTERVAL
    fi
    echo ""
done

echo "=========================================="
echo "  所有步骤完成!"
echo "  结束时间: $(date '+%Y-%m-%d %H:%M:%S')"
echo "  总步骤数: $TOTAL_STEPS"
echo "  每步间隔: ${SLEEP_INTERVAL}秒"
echo "  预计总耗时: ~$((TOTAL_STEPS * (SLEEP_INTERVAL + 1)))秒"
echo "=========================================="
