pub(super) fn translate_en_us(key: &str) -> Option<&'static str> {
    match key {
        "agent.toolCatalog.productivity.params.cwd" => {
            Some("Working directory; defaults to the current application directory.")
        }
        "agent.toolCatalog.productivity.params.path" => Some("Project file or directory path."),
        "agent.toolCatalog.productivity.params.fileRef" => {
            Some("Input file from a linked project directory or this chat session.")
        }
        "agent.toolCatalog.productivity.params.fileRef.kind" => {
            Some("File source: workspace_path or session_attachment.")
        }
        "agent.toolCatalog.productivity.params.fileRef.path" => {
            Some("Path inside a linked project directory.")
        }
        "agent.toolCatalog.productivity.params.fileRef.attachmentCode" => {
            Some("Attachment code shown in this chat's attachment context.")
        }
        "agent.toolCatalog.productivity.params.outputPath" => {
            Some("New output file; must not exist. Source files are never overwritten.")
        }
        "agent.toolCatalog.productivity.params.branch" => Some("Branch name."),
        "agent.toolCatalog.productivity.params.name" => Some("Worktree or worksheet name."),
        "agent.toolCatalog.productivity.params.row" => Some("One-based row."),
        "agent.toolCatalog.productivity.params.column" => Some("One-based column."),
        "agent.toolCatalog.productivity.params.value" => {
            Some("Literal cell value; text beginning with = remains text.")
        }
        "agent.toolCatalog.productivity.params.formula" => {
            Some("Cell formula to save in the workbook.")
        }
        "agent.toolCatalog.productivity.params.sheets" => {
            Some("Worksheets and cell changes. Existing workbook input requires outputPath.")
        }
        "agent.toolCatalog.productivity.worktree.list.name" => Some("List worktrees"),
        "agent.toolCatalog.productivity.worktree.list.description" => {
            Some("List worktrees in the linked repository when needed.")
        }
        "agent.toolCatalog.productivity.worktree.list.result" => {
            Some("Returns bounded execution results or the new artifact path.")
        }
        "agent.toolCatalog.productivity.params.startPoint" => {
            Some("Starting revision, defaults to HEAD.")
        }
        "agent.toolCatalog.productivity.worktree.create.name" => Some("Create and select worktree"),
        "agent.toolCatalog.productivity.worktree.create.description" => Some(
            "Create a new branch and managed sibling worktree, then select it for the next model step. Requires a project-linked application.",
        ),
        "agent.toolCatalog.productivity.worktree.create.result" => {
            Some("Returns bounded execution results or the new artifact path.")
        }
        "agent.toolCatalog.productivity.worktree.select.name" => Some("Select worktree"),
        "agent.toolCatalog.productivity.worktree.select.description" => Some(
            "Select an existing worktree of the linked repository for subsequent tool execution.",
        ),
        "agent.toolCatalog.productivity.worktree.select.result" => {
            Some("Returns bounded execution results or the new artifact path.")
        }
        "agent.toolCatalog.productivity.params.limit" => Some("Maximum returned items."),
        "agent.toolCatalog.productivity.params.startParagraph" => {
            Some("Zero-based paragraph offset.")
        }
        "agent.toolCatalog.productivity.word.read.name" => Some("Read Word document"),
        "agent.toolCatalog.productivity.word.read.description" => Some(
            "Read DOCX body paragraphs and table text with pagination from a linked project file or session attachment.",
        ),
        "agent.toolCatalog.productivity.word.read.result" => {
            Some("Returns bounded execution results or the new artifact path.")
        }
        "agent.toolCatalog.productivity.params.paragraphs" => {
            Some("Document paragraphs and heading levels.")
        }
        "agent.toolCatalog.productivity.params.table" => Some("Optional rows of text cells."),
        "agent.toolCatalog.productivity.word.create.name" => Some("Create Word document"),
        "agent.toolCatalog.productivity.word.create.description" => Some(
            "Create a new DOCX with paragraphs, headings and an optional table at an unused output path.",
        ),
        "agent.toolCatalog.productivity.word.create.result" => {
            Some("Returns bounded execution results or the new artifact path.")
        }
        "agent.toolCatalog.productivity.params.find" => {
            Some("Exact text to find within each paragraph.")
        }
        "agent.toolCatalog.productivity.params.replace" => Some("Replacement text."),
        "agent.toolCatalog.productivity.params.expectedCount" => {
            Some("Required match count; defaults to 1.")
        }
        "agent.toolCatalog.productivity.word.replace_text.name" => Some("Replace Word text"),
        "agent.toolCatalog.productivity.word.replace_text.description" => Some(
            "Replace exact text in document paragraphs from a linked project file or session attachment, preserve other document content, and save a new DOCX to outputPath. Expected match count must agree.",
        ),
        "agent.toolCatalog.productivity.word.replace_text.result" => {
            Some("Returns bounded execution results or the new artifact path.")
        }
        "agent.toolCatalog.productivity.params.sheet" => {
            Some("Worksheet name; omit to list sheets.")
        }
        "agent.toolCatalog.productivity.params.startRow" => Some("First row, one-based."),
        "agent.toolCatalog.productivity.params.startColumn" => Some("First column, one-based."),
        "agent.toolCatalog.productivity.params.rows" => Some("Number of rows, up to 200."),
        "agent.toolCatalog.productivity.params.columns" => Some("Number of columns, up to 100."),
        "agent.toolCatalog.productivity.excel.read.name" => Some("Read Excel workbook"),
        "agent.toolCatalog.productivity.excel.read.description" => Some(
            "List worksheets or read cached cell values and formulas from an XLSX range in a linked project file or session attachment.",
        ),
        "agent.toolCatalog.productivity.excel.read.result" => {
            Some("Returns bounded execution results or the new artifact path.")
        }
        "agent.toolCatalog.productivity.excel.write.name" => Some("Write Excel workbook"),
        "agent.toolCatalog.productivity.excel.write.description" => Some(
            "Create an XLSX at path, or update worksheet cells and formulas in a copy of a project workbook or session attachment and save it to outputPath.",
        ),
        "agent.toolCatalog.productivity.excel.write.result" => {
            Some("Returns bounded execution results or the new artifact path.")
        }
        "agent.toolCatalog.domains.worktree.name" => Some("Worktree tools"),
        "agent.toolCatalog.domains.worktree.description" => {
            Some("Discover, create and select Git worktrees on demand.")
        }
        "agent.toolCatalog.domains.word.name" => Some("Word tools"),
        "agent.toolCatalog.domains.word.description" => {
            Some("Read, create and edit DOCX documents.")
        }
        "agent.toolCatalog.domains.excel.name" => Some("Excel tools"),
        "agent.toolCatalog.domains.excel.description" => {
            Some("Read and write XLSX worksheets and formulas.")
        }
        _ => None,
    }
}
pub(super) fn translate_zh_cn(key: &str) -> Option<&'static str> {
    match key {
        "agent.toolCatalog.productivity.params.cwd" => Some("工作目录；缺省使用当前应用目录。"),
        "agent.toolCatalog.productivity.params.path" => Some("项目文件或目录路径。"),
        "agent.toolCatalog.productivity.params.fileRef" => {
            Some("输入文件：已关联项目目录中的文件，或当前会话附件。")
        }
        "agent.toolCatalog.productivity.params.fileRef.kind" => {
            Some("文件来源：workspace_path 或 session_attachment。")
        }
        "agent.toolCatalog.productivity.params.fileRef.path" => {
            Some("已关联项目目录中的文件路径。")
        }
        "agent.toolCatalog.productivity.params.fileRef.attachmentCode" => {
            Some("当前会话附件上下文中的附件编号。")
        }
        "agent.toolCatalog.productivity.params.outputPath" => {
            Some("新的输出文件，必须不存在；不覆盖源文件。")
        }
        "agent.toolCatalog.productivity.params.branch" => Some("分支名称。"),
        "agent.toolCatalog.productivity.params.name" => Some("工作区或工作表名称。"),
        "agent.toolCatalog.productivity.params.row" => Some("行号，从 1 开始。"),
        "agent.toolCatalog.productivity.params.column" => Some("列号，从 1 开始。"),
        "agent.toolCatalog.productivity.params.value" => {
            Some("单元格值；以 = 开头的文本仍作为文本保存。")
        }
        "agent.toolCatalog.productivity.params.formula" => Some("写入工作簿的单元格公式。"),
        "agent.toolCatalog.productivity.params.sheets" => {
            Some("工作表及单元格变更；编辑现有工作簿时必须提供 outputPath。")
        }
        "agent.toolCatalog.productivity.worktree.list.name" => Some("查询工作区"),
        "agent.toolCatalog.productivity.worktree.list.description" => {
            Some("按需查询关联仓库的工作区。")
        }
        "agent.toolCatalog.productivity.worktree.list.result" => {
            Some("返回受限长度的执行结果或新文件路径。")
        }
        "agent.toolCatalog.productivity.params.startPoint" => Some("起始提交，默认 HEAD。"),
        "agent.toolCatalog.productivity.worktree.create.name" => Some("创建并使用工作区"),
        "agent.toolCatalog.productivity.worktree.create.description" => {
            Some("创建新分支和托管工作区，并在下一模型步骤使用它；需要关联项目应用。")
        }
        "agent.toolCatalog.productivity.worktree.create.result" => {
            Some("返回受限长度的执行结果或新文件路径。")
        }
        "agent.toolCatalog.productivity.worktree.select.name" => Some("切换工作区"),
        "agent.toolCatalog.productivity.worktree.select.description" => {
            Some("切换到关联仓库的已有工作区，供后续工具执行使用。")
        }
        "agent.toolCatalog.productivity.worktree.select.result" => {
            Some("返回受限长度的执行结果或新文件路径。")
        }
        "agent.toolCatalog.productivity.params.limit" => Some("最多返回条数。"),
        "agent.toolCatalog.productivity.params.startParagraph" => Some("段落起点，从 0 开始。"),
        "agent.toolCatalog.productivity.word.read.name" => Some("读取 Word 文档"),
        "agent.toolCatalog.productivity.word.read.description" => {
            Some("分页读取已关联项目文件或会话附件中的 DOCX 正文段落和表格文字。")
        }
        "agent.toolCatalog.productivity.word.read.result" => {
            Some("返回受限长度的执行结果或新文件路径。")
        }
        "agent.toolCatalog.productivity.params.paragraphs" => Some("正文段落和标题级别。"),
        "agent.toolCatalog.productivity.params.table" => Some("可选表格，每行由文本单元格组成。"),
        "agent.toolCatalog.productivity.word.create.name" => Some("创建 Word 文档"),
        "agent.toolCatalog.productivity.word.create.description" => {
            Some("创建包含正文、标题和可选表格的 DOCX，输出路径必须未被占用。")
        }
        "agent.toolCatalog.productivity.word.create.result" => {
            Some("返回受限长度的执行结果或新文件路径。")
        }
        "agent.toolCatalog.productivity.params.find" => Some("在每个段落中精确查找的文本。"),
        "agent.toolCatalog.productivity.params.replace" => Some("替换后的文本。"),
        "agent.toolCatalog.productivity.params.expectedCount" => Some("预期匹配次数，默认 1。"),
        "agent.toolCatalog.productivity.word.replace_text.name" => Some("替换 Word 文本"),
        "agent.toolCatalog.productivity.word.replace_text.description" => Some(
            "替换已关联项目文件或会话附件中 DOCX 正文段落的精确文本，保留其他文档内容，另存到 outputPath；匹配次数必须一致。",
        ),
        "agent.toolCatalog.productivity.word.replace_text.result" => {
            Some("返回受限长度的执行结果或新文件路径。")
        }
        "agent.toolCatalog.productivity.params.sheet" => Some("工作表名称；省略时列出工作表。"),
        "agent.toolCatalog.productivity.params.startRow" => Some("起始行，从 1 开始。"),
        "agent.toolCatalog.productivity.params.startColumn" => Some("起始列，从 1 开始。"),
        "agent.toolCatalog.productivity.params.rows" => Some("读取行数，最多 200。"),
        "agent.toolCatalog.productivity.params.columns" => Some("读取列数，最多 100。"),
        "agent.toolCatalog.productivity.excel.read.name" => Some("读取 Excel 工作簿"),
        "agent.toolCatalog.productivity.excel.read.description" => {
            Some("查询已关联项目文件或会话附件中的 XLSX 工作表，按区域读取单元格缓存值和公式。")
        }
        "agent.toolCatalog.productivity.excel.read.result" => {
            Some("返回受限长度的执行结果或新文件路径。")
        }
        "agent.toolCatalog.productivity.excel.write.name" => Some("写入 Excel 工作簿"),
        "agent.toolCatalog.productivity.excel.write.description" => Some(
            "在 path 创建 XLSX，或修改项目工作簿、会话附件中的工作表单元格与公式，并另存到 outputPath。",
        ),
        "agent.toolCatalog.productivity.excel.write.result" => {
            Some("返回受限长度的执行结果或新文件路径。")
        }
        "agent.toolCatalog.domains.worktree.name" => Some("工作区工具"),
        "agent.toolCatalog.domains.worktree.description" => {
            Some("按需查询、创建和切换 Git 工作区。")
        }
        "agent.toolCatalog.domains.word.name" => Some("Word 工具"),
        "agent.toolCatalog.domains.word.description" => Some("读取、创建和编辑 DOCX 文档。"),
        "agent.toolCatalog.domains.excel.name" => Some("Excel 工具"),
        "agent.toolCatalog.domains.excel.description" => Some("读写 XLSX 工作表和公式。"),
        _ => None,
    }
}
