import {
  FileArchive,
  RefreshCcw,
  Settings2,
} from "lucide-react";

import {
  Field,
  SelectInput,
  SurfaceCard,
  TextArea,
  TextInput,
} from "@/shared";
import { Button } from "@/shared/ui/Button";

import {
  DiagnosticPreview,
  SectionTitle,
} from "@/domain/rag/components/RagShared";

export function RagDiagnosticsTab({ onNotice }: { onNotice: (notice: string) => void }) {
  return (
    <div className="grid gap-5 xl:grid-cols-3">
      <SurfaceCard>
        <SectionTitle
          icon={<Settings2 size={17} aria-hidden="true" />}
          title="调试配置"
        />
        <div className="mt-4 grid gap-3">
          <Field label="配置标识">
            <SelectInput defaultValue="project-doc-related-v1">
              <option value="project-doc-related-v1">项目文档相关性配置</option>
              <option value="memory-related-v1">记忆相关性配置</option>
              <option value="skill-related-v1">技能相关性配置</option>
            </SelectInput>
          </Field>
          <Button variant="secondary" onClick={() => onNotice("配置调试响应预览已刷新。")}>
            加载配置
          </Button>
        </div>
        <DiagnosticPreview rows={[["路由", "全文、向量"], ["策略", "文档问答上下文包"], ["告警", "2"]]} />
      </SurfaceCard>

      <SurfaceCard>
        <SectionTitle
          icon={<FileArchive size={17} aria-hidden="true" />}
          title="调试来源"
        />
        <div className="mt-4 grid gap-3">
          <Field label="来源编码">
            <TextInput defaultValue="docs/features/rag-search-context-pack" />
          </Field>
          <Button variant="secondary" onClick={() => onNotice("来源调试响应预览已刷新。")}>
            加载来源
          </Button>
        </div>
        <DiagnosticPreview rows={[["状态", "活跃"], ["文本单元", "348"], ["上次索引", "8 分钟前"]]} />
      </SurfaceCard>

      <SurfaceCard>
        <SectionTitle
          icon={<RefreshCcw size={17} aria-hidden="true" />}
          title="重试索引"
        />
        <div className="mt-4 grid gap-3">
          <Field label="阶段">
            <SelectInput defaultValue="vectorIndex">
              <option value="fulltextIndex">全文索引</option>
              <option value="vectorIndex">向量索引</option>
              <option value="extractTree">提取结构树</option>
              <option value="summarizeTree">摘要结构树</option>
              <option value="treeFulltextIndex">结构树全文索引</option>
              <option value="treeVectorIndex">结构树向量索引</option>
              <option value="persistTree">持久化结构树</option>
            </SelectInput>
          </Field>
          <Field label="原因">
            <TextArea className="min-h-24" defaultValue="配置校验通过后，补齐缺失的向量索引。" />
          </Field>
          <Button onClick={() => onNotice("索引重试请求已加入队列。")}>
            加入重试队列
          </Button>
        </div>
      </SurfaceCard>
    </div>
  );
}
