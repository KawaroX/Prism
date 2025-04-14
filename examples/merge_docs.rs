use prism::ThreeWayMerger;
use prism::error::Result;
use prism::merge::Resolution;

fn main() -> Result<()> {
    // 创建合并引擎
    let merger = ThreeWayMerger::new();

    println!("正在执行文档合并...");

    let (merged_doc, conflicts) = merger.merge_documents(
        "base_document.docx",
        "modified_document1.docx",
        "modified_document2.docx",
        None,
        Some("merged_document.docx"),
    )?;

    // 2. 检查是否有冲突
    if conflicts.is_empty() {
        println!("合并成功完成，没有冲突。");
        return Ok(());
    }

    // 3. 显示冲突信息
    println!("检测到 {} 个冲突:", conflicts.len());
    for (i, conflict) in conflicts.iter().enumerate() {
        println!("冲突 #{}: 类型: {:?}", i + 1, conflict.conflict_type);

        // 显示冲突内容
        match &conflict.content1 {
            prism::merge::ConflictContent::Paragraph(text) => {
                println!("  文档1: {}", text);
            }
            _ => {}
        }

        match &conflict.content2 {
            prism::merge::ConflictContent::Paragraph(text) => {
                println!("  文档2: {}", text);
            }
            _ => {}
        }

        println!();
    }

    // 4. 收集用户解决方案（在实际应用中会有UI交互）
    let mut resolutions = Vec::new();

    // 模拟用户选择
    for (i, _) in conflicts.iter().enumerate() {
        println!("请选择如何解决冲突 #{}:", i + 1);
        println!("1. 使用文档1的版本");
        println!("2. 使用文档2的版本");
        println!("3. 合并两者");

        // 这里应该读取用户输入，现在我们假设用户选择第一个选项
        let choice = 1;

        let resolution = match choice {
            1 => Resolution::UseFirst,
            2 => Resolution::UseSecond,
            3 => Resolution::MergeBoth,
            _ => Resolution::UseFirst, // 默认选项
        };

        resolutions.push(resolution);
    }

    // 5. 应用用户选择的解决方案并保存最终结果
    merger.resolve_all_conflicts(
        &merged_doc,
        &conflicts,
        &resolutions,
        "final_merged.docx", // 保存最终结果
    )?;

    println!("冲突已解决，最终文档已保存为 final_merged.docx");

    Ok(())
}
