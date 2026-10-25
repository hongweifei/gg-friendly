#!/bin/bash
# compare the feature surface of the original and the current version
cd /home/weifei/Projects/gg-friendly || exit 1

pass=0
fail=0

check() {
  local desc="$1" orig_file="$2" pattern="$3" now_file="$4"
  if git show "HEAD:$orig_file" 2>/dev/null | grep -q "$pattern" \
     && grep -q "$pattern" "$now_file" 2>/dev/null; then
    printf "  ✓ %s\n" "$desc"
    pass=$((pass + 1))
  else
    printf "  ✗ %s\n" "$desc"
    fail=$((fail + 1))
  fi
}

echo "改动列表里的对象"
check "分支标签（可拖拽）" app/objects/RevisionObject.svelte BookmarkObject app/objects/ChangeCard.svelte
check "标签对象" app/objects/RevisionObject.svelte TagObject app/objects/ChangeCard.svelte
check "工作副本标记" app/objects/RevisionObject.svelte WorkspaceObject app/objects/ChangeCard.svelte
check "标识" app/objects/RevisionObject.svelte IdSpan app/objects/ChangeCard.svelte
check "作者" app/objects/RevisionObject.svelte AuthorSpan app/objects/ChangeCard.svelte

echo "详情面板的操作"
for op in onEdit onNewChild onDescribe onSquash onRestore; do
  check "mutator.$op" app/RevisionPane.svelte "mutator.$op" app/RevisionPane.svelte
done

echo "拖放与右键"
check "拖拽源" app/objects/Object.svelte 'draggable={operand != null}' app/objects/Object.svelte
check "右键菜单" app/objects/Object.svelte 'on:contextmenu' app/objects/Object.svelte
check "双击编辑" app/objects/Object.svelte onDoubleClick app/objects/Object.svelte
check "拖放落点" app/objects/Zone.svelte onDrop app/objects/Zone.svelte

echo "列表工具栏"
check "查询下拉框" app/LogPane.svelte SelectWidget app/LogPane.svelte
check "自定义查询输入框" app/LogPane.svelte 'type="text"' app/LogPane.svelte
check "保存查询" app/LogPane.svelte handleSavePreset app/LogPane.svelte
check "删除查询" app/LogPane.svelte handleDeletePreset app/LogPane.svelte
check "虚拟滚动" app/GraphLog.svelte visibleSlice app/GraphLog.svelte

echo ""
echo "通过 $pass 项，缺失 $fail 项"
